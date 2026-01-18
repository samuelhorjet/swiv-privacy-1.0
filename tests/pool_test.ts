import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SwivPrivacy } from "../target/types/swiv_privacy";
import {
  PublicKey,
  SystemProgram,
  Keypair,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import * as fs from "fs";
import * as path from "path";
import {
  SEED_BET,
  SEED_POOL,
  SEED_GLOBAL_CONFIG,
  createCommitment,
  performUndelegate,
  sleep,
} from "./utils";
import { MAGIC_PROGRAM_ID } from "@magicblock-labs/ephemeral-rollups-sdk";

const KEYS_DIR = path.join(__dirname, "keys");
if (!fs.existsSync(KEYS_DIR)) fs.mkdirSync(KEYS_DIR);

function loadOrGenerateKeypair(name: string): Keypair {
  const filePath = path.join(KEYS_DIR, `${name}.json`);
  if (fs.existsSync(filePath)) {
    return Keypair.fromSecretKey(
      new Uint8Array(JSON.parse(fs.readFileSync(filePath, "utf-8")))
    );
  } else {
    const kp = Keypair.generate();
    fs.writeFileSync(filePath, JSON.stringify(Array.from(kp.secretKey)));
    return kp;
  }
}

async function waitForOwnership(
  connection: anchor.web3.Connection,
  account: PublicKey,
  expectedOwner: PublicKey,
  timeout = 30000
) {
  const start = Date.now();
  let currentOwner = null;

  while (Date.now() - start < timeout) {
    const info = await connection.getAccountInfo(account, "confirmed");
    if (info) {
      currentOwner = info.owner;
      if (info.owner.equals(expectedOwner)) {
        return true;
      }
    }
    await sleep(2000);
    console.log(
      `      ⏳ Waiting for L1 ownership... Current: ${
        currentOwner ? currentOwner.toBase58().slice(0, 6) : "null"
      } vs Expected: ${expectedOwner.toBase58().slice(0, 6)}`
    );
  }
  return false;
}

describe("2. Pool Test (Multi-Bet + Refund Scenario)", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.SwivPrivacy as Program<SwivPrivacy>;
  const admin = (provider.wallet as anchor.Wallet).payer;

  const users = [
    loadOrGenerateKeypair("userP1"),
    loadOrGenerateKeypair("userP2"),
    loadOrGenerateKeypair("userP3"),
    loadOrGenerateKeypair("userP4"),
    loadOrGenerateKeypair("userP5"),
  ];

  let usdcMint: PublicKey;
  let userAtas: PublicKey[] = [];

  let globalConfigPda: PublicKey;
  let poolPda: PublicKey;
  let vaultPda: PublicKey;

  const POOL_NAME = `Refund-Test-${Math.floor(Math.random() * 99999)}`;
  const TARGET_PRICE = new anchor.BN(200_000_000); // 200.00 USDC

  const randId = Math.floor(Math.random() * 1000);

  const betsConfig = [
    // 🟢 WINNER (User 1 - Bet A) - Will be UPDATED
    {
      userIdx: 0,
      id: `u1_betA_${randId}`,
      price: TARGET_PRICE,
      salt: Keypair.generate().publicKey.toBuffer(),
      type: "WINNER_UPDATE",
    },

    // 🟠 PARTIAL/CLOSE (User 1 - Bet B)
    {
      userIdx: 0,
      id: `u1_betB_${randId}`,
      price: TARGET_PRICE.add(new anchor.BN(10)),
      salt: Keypair.generate().publicKey.toBuffer(),
      type: "PARTIAL",
    },

    // 🛑 REFUND CANDIDATE (User 1 - Bet C) - "The Sneaky Bet" (Will NOT be Revealed)
    {
      userIdx: 0,
      id: `u1_betC_${randId}`,
      price: TARGET_PRICE,
      salt: Keypair.generate().publicKey.toBuffer(),
      type: "REFUND_TEST",
    },

    // 🟢 WINNER (User 2)
    {
      userIdx: 1,
      id: `u2_bet_${randId}`,
      price: TARGET_PRICE,
      salt: Keypair.generate().publicKey.toBuffer(),
      type: "WINNER",
    },

    // 🟠 PARTIAL/MID (User 3)
    {
      userIdx: 2,
      id: `u3_bet_${randId}`,
      price: TARGET_PRICE.add(new anchor.BN(50)),
      salt: Keypair.generate().publicKey.toBuffer(),
      type: "PARTIAL",
    },

    // 🔴 TOTAL LOSER (User 4)
    {
      userIdx: 3,
      id: `u4_bet_${randId}`,
      price: TARGET_PRICE.add(new anchor.BN(500000)),
      salt: Keypair.generate().publicKey.toBuffer(),
      type: "TOTAL_LOSS",
    },

    // 🟢 WINNER (User 5)
    {
      userIdx: 4,
      id: `u5_bet_${randId}`,
      price: TARGET_PRICE,
      salt: Keypair.generate().publicKey.toBuffer(),
      type: "WINNER",
    },
  ];

  const betPdas: PublicKey[] = [];
  let poolEndTime = 0;

  it("Setup: Environment & Funding", async () => {
    [globalConfigPda] = PublicKey.findProgramAddressSync(
      [SEED_GLOBAL_CONFIG],
      program.programId
    );

    usdcMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );

    userAtas = [];
    for (const user of users) {
      const bal = await provider.connection.getBalance(user.publicKey);
      if (bal < 0.1 * LAMPORTS_PER_SOL) {
        const tx = new anchor.web3.Transaction().add(
          SystemProgram.transfer({
            fromPubkey: admin.publicKey,
            toPubkey: user.publicKey,
            lamports: 0.1 * LAMPORTS_PER_SOL,
          })
        );
        await provider.sendAndConfirm(tx);
      }
      const ata = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        admin,
        usdcMint,
        user.publicKey
      );
      userAtas.push(ata.address);

      await mintTo(
        provider.connection,
        admin,
        usdcMint,
        ata.address,
        admin,
        1000 * 1_000_000
      );
    }
    console.log("    ✅ Setup Complete");
  });

  it("Create Pool", async () => {
    const now = Math.floor(Date.now() / 1000);
    const START_TIME = new anchor.BN(now);
    const DURATION = 80; // slightly longer for safety
    const END_TIME = START_TIME.add(new anchor.BN(DURATION));
    poolEndTime = now + DURATION;

    [poolPda] = PublicKey.findProgramAddressSync(
      [SEED_POOL, Buffer.from(POOL_NAME)],
      program.programId
    );
    [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool_vault"), poolPda.toBuffer()],
      program.programId
    );

    const adminAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      admin.publicKey
    );

    await program.methods
      .createPool(
        POOL_NAME,
        "Hybrid Test Refund",
        START_TIME,
        END_TIME,
        new anchor.BN(500),
        new anchor.BN(1000)
      )
      .accountsPartial({
        globalConfig: globalConfigPda,
        pool: poolPda,
        poolVault: vaultPda,
        tokenMint: usdcMint,
        admin: admin.publicKey,
        adminTokenAccount: adminAta.address,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log(`    ✅ Pool Created: ${POOL_NAME}`);
  });

  it("Place Bets (User 1 x3, Others x1)", async () => {
    const betAmount = new anchor.BN(100 * 1_000_000);

    for (let i = 0; i < betsConfig.length; i++) {
      if (i > 0) await sleep(50);

      const bet = betsConfig[i];
      const user = users[bet.userIdx];
      const commitment = createCommitment(bet.price, bet.salt);

      const [betPda] = PublicKey.findProgramAddressSync(
        [
          SEED_BET,
          poolPda.toBuffer(),
          user.publicKey.toBuffer(),
          Buffer.from(bet.id),
        ],
        program.programId
      );
      betPdas.push(betPda);

      await program.methods
        .placeBet(betAmount, Array.from(commitment), bet.id)
        .accountsPartial({
          user: user.publicKey,
          globalConfig: globalConfigPda,
          pool: poolPda,
          poolVault: vaultPda,
          userTokenAccount: userAtas[bet.userIdx],
          userBet: betPda,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([user])
        .rpc();

      console.log(
        `    ✅ Bet Placed [${bet.type}]: User ${bet.userIdx + 1} | ID: ${
          bet.id
        }`
      );
    }
  });

  it("Delegate, Update (Bet A), Reveal (Skip Bet C)", async () => {
    console.log("    > Delegating all bets...");
    for (let i = 0; i < betsConfig.length; i++) {
      const bet = betsConfig[i];
      const user = users[bet.userIdx];

      await program.methods
        .delegateBet(bet.id)
        .accounts({
          user: user.publicKey,
          pool: poolPda,
          userBet: betPdas[i],
        })
        .signers([user])
        .rpc();
    }
    await sleep(4000);

    const erConnection = new anchor.web3.Connection(
      "https://devnet.magicblock.app"
    );

    console.log("    > User 1 Updating Bet A (Single Op)...");
    const betA_Index = betsConfig.findIndex((b) => b.type === "WINNER_UPDATE");
    const user1 = users[0];

    const erProvider1 = new anchor.AnchorProvider(
      erConnection,
      new anchor.Wallet(user1),
      {}
    );
    const erProgram1 = new anchor.Program(program.idl, erProvider1);

    try {
      await erProgram1.methods
        .updateBet(TARGET_PRICE)
        .accounts({
          user: user1.publicKey,
          userBet: betPdas[betA_Index],
          pool: null,
        })
        .rpc();
      console.log("    ✅ User 1 Updated Bet A successfully");
    } catch (e) {
      console.error("    ❌ User 1 Update Fail:", e);
    }

    console.log("    > Revealing bets...");
    for (let i = 0; i < betsConfig.length; i++) {
      const bet = betsConfig[i];

      if (bet.type === "WINNER_UPDATE") {
        console.log(`      - Skipping ${bet.id} (Already Revealed via Update)`);
        continue;
      }
      if (bet.type === "REFUND_TEST") {
        console.log(
          `      🛑 SKIPPING REVEAL for ${bet.id} (Testing Refund Logic)`
        );
        continue;
      }

      const user = users[bet.userIdx];
      const erProvider = new anchor.AnchorProvider(
        erConnection,
        new anchor.Wallet(user),
        {}
      );
      const erProgram = new anchor.Program(program.idl, erProvider);

      try {
        await erProgram.methods
          .revealBet(bet.price, Array.from(bet.salt))
          .accounts({ user: user.publicKey, userBet: betPdas[i] })
          .rpc();
        console.log(`      - Revealed: ${bet.id}`);
      } catch (e) {
        console.error(`      - Fail Reveal ${bet.id}:`, e);
      }
      await sleep(100);
    }
  });

  it("Wait for Expiry & Hybrid Undelegate", async () => {
    const now = Math.floor(Date.now() / 1000);
    const timeLeft = poolEndTime - now;
    if (timeLeft > 0) {
      console.log(`    ⏳ Waiting ${timeLeft + 5}s for expiry...`);
      await sleep((timeLeft + 5) * 1000);
    }

    const erConnection = new anchor.web3.Connection(
      "https://devnet.magicblock.app"
    );

    const betA_Index = betsConfig.findIndex((b) => b.type === "WINNER_UPDATE");
    console.log(
      `    🛠️  User 1 Single Undelegate (${betsConfig[betA_Index].id})...`
    );

    try {
      await performUndelegate(
        new anchor.Program(
          program.idl,
          new anchor.AnchorProvider(
            erConnection,
            new anchor.Wallet(users[0]),
            {}
          )
        ),
        erConnection,
        users[0],
        betsConfig[betA_Index].id,
        poolPda,
        betPdas[betA_Index]
      );
      console.log("    ✅ Single Undelegate Success");
    } catch (e) {
      console.error("    ❌ Single Undelegate Failed", e);
    }

    console.log("    🛠️  Admin Batch Undelegating Remaining...");
    const batchAccounts = betPdas
      .filter((_, i) => i !== betA_Index)
      .map((pubkey) => ({
        pubkey,
        isWritable: true,
        isSigner: false,
      }));

    try {
      await new anchor.Program(
        program.idl,
        new anchor.AnchorProvider(erConnection, new anchor.Wallet(admin), {})
      ).methods
        .batchUndelegateBets()
        .accounts({
          payer: admin.publicKey,
          pool: poolPda,
          magicProgram: MAGIC_PROGRAM_ID,
        })
        .remainingAccounts(batchAccounts)
        .rpc();
      console.log("    ✅ Batch Undelegate Sent");
    } catch (e) {
      console.error("    ❌ Batch Undelegate Failed:", e);
    }

    await sleep(2000);
    console.log("    🔍 Verifying L1 Ownership...");
    for (let i = 0; i < betsConfig.length; i++) {
      const isBack = await waitForOwnership(
        provider.connection,
        betPdas[i],
        program.programId
      );
      if (isBack) {
        console.log(`      ✅ Bet ${i} confirmed on L1`);
      }
    }
  });

  it("Resolve, Calculate & Claim (Standard Flow)", async () => {
    await program.methods
      .resolvePool(TARGET_PRICE)
      .accountsPartial({
        admin: admin.publicKey,
        globalConfig: globalConfigPda,
        pool: poolPda,
      })
      .rpc();
    console.log("    ✅ Pool Resolved");
    await sleep(6000); 

    console.log("    🛠️  Calculating Outcomes (Skipping Unrevealed Bet C)...");

    const calcAccounts = betPdas
      .filter((_, i) => betsConfig[i].type !== "REFUND_TEST")
      .map((pubkey) => ({
        pubkey,
        isWritable: true,
        isSigner: false,
      }));

    await program.methods
      .batchCalculateOutcome()
      .accountsPartial({ admin: admin.publicKey, pool: poolPda })
      .remainingAccounts(calcAccounts)
      .rpc();
    console.log("    ✅ Batch Calculation Complete");

    const config = await program.account.globalConfig.fetch(globalConfigPda);
    const treasuryInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      config.treasuryWallet,
      true
    );

    await program.methods
      .finalizeWeights()
      .accountsPartial({
        admin: admin.publicKey,
        globalConfig: globalConfigPda,
        pool: poolPda,
        poolVault: vaultPda,
        treasuryWallet: treasuryInfo.address,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log("    ✅ Weights Finalized");

    console.log("\n    💸 --- CLAIMING REWARDS ---");
    for (let i = 0; i < betsConfig.length; i++) {
      const bet = betsConfig[i];
      if (bet.type === "REFUND_TEST") continue; 

      const user = users[bet.userIdx];
      const preBal = (
        await provider.connection.getTokenAccountBalance(userAtas[bet.userIdx])
      ).value.uiAmount;

      try {
        await program.methods
          .claimReward()
          .accountsPartial({
            user: user.publicKey,
            pool: poolPda,
            poolVault: vaultPda,
            userBet: betPdas[i],
            userTokenAccount: userAtas[bet.userIdx],
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([user])
          .rpc();

        const postBal = (
          await provider.connection.getTokenAccountBalance(
            userAtas[bet.userIdx]
          )
        ).value.uiAmount;
        const payout = postBal - preBal;

        let label = "Lost";
        if (payout > 90) label = "WINNER 🎉";
        else if (payout > 0) label = "PARTIAL 💰";
        else if (bet.type === "TOTAL_LOSS") label = "TOTAL LOSS 💀";

        console.log(
          `    Bet ${i} (${bet.id}): Payout ${payout.toFixed(
            2
          )} USDC [${label}]`
        );
      } catch (e) {
        console.log(`    Bet ${i} (${bet.id}): No Reward (Lost/Claimed)`);
      }
    }
  });

  it("Refund Unrevealed Bet (User 1 Bet C)", async () => {
    console.log("\n    🔄 --- TESTING REFUND ---");

    const refundIndex = betsConfig.findIndex((b) => b.type === "REFUND_TEST");
    const refundBetPda = betPdas[refundIndex];
    const user = users[betsConfig[refundIndex].userIdx];
    const userAta = userAtas[betsConfig[refundIndex].userIdx];

    const preBal = (await provider.connection.getTokenAccountBalance(userAta))
      .value.uiAmount;

    const config = await program.account.globalConfig.fetch(globalConfigPda);
    const correctTreasuryWallet = config.treasuryWallet;

    const treasuryAtaInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      correctTreasuryWallet,
      true
    );
    const correctTreasuryAta = treasuryAtaInfo.address;

    const preTreasury = (
      await provider.connection.getTokenAccountBalance(correctTreasuryAta)
    ).value.uiAmount;

    console.log(`    > Refunding Bet C (${betsConfig[refundIndex].id})...`);

    try {
      await program.methods
        .refundBet()
        .accountsPartial({
          user: user.publicKey,
          userBet: refundBetPda,
          globalConfig: globalConfigPda,
          treasuryWallet: correctTreasuryWallet, 
          treasuryTokenAccount: correctTreasuryAta, 
          userTokenAccount: userAta,
          pool: poolPda,
          poolVault: vaultPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();

      const postUser = (
        await provider.connection.getTokenAccountBalance(userAta)
      ).value.uiAmount;
      const postTreasury = (
        await provider.connection.getTokenAccountBalance(correctTreasuryAta)
      ).value.uiAmount;

      console.log(`\n    👻 Ghost Stats (Refunded):`);
      console.log(
        `       User Received: ${(postUser - preBal).toFixed(2)} USDC`
      );
      console.log(
        `       Treasury Fee:  ${(postTreasury - preTreasury).toFixed(2)} USDC`
      );
    } catch (e) {
      console.error("    ❌ Refund Failed:", e);
      throw e;
    }
  });
});
