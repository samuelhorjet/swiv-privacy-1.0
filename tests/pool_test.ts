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

describe("2. Pool Test (Mixed Single & Batch Ops)", () => {
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
  let treasuryAta: PublicKey;
  let userAtas: PublicKey[] = [];

  let globalConfigPda: PublicKey;
  let poolPda: PublicKey;
  let vaultPda: PublicKey;

  const POOL_NAME = `Pool-TEST-${Math.floor(Math.random() * 99999)}`;
  const TARGET_PRICE = new anchor.BN(200_000_000);

  const randId = Math.floor(Math.random() * 1000);
  const requestIds = users.map((_, i) => `p${randId}_${i}`);
  const salts = users.map(() => Keypair.generate().publicKey.toBuffer());
  const betPdas: PublicKey[] = [];

  const predictions = [
    TARGET_PRICE,
    TARGET_PRICE,
    TARGET_PRICE.add(new anchor.BN(400)),
    TARGET_PRICE.add(new anchor.BN(20000)),
    TARGET_PRICE.sub(new anchor.BN(2000)),
  ];

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

    const config = await program.account.globalConfig.fetch(globalConfigPda);
    const treasuryInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      config.treasuryWallet,
      true
    );
    treasuryAta = treasuryInfo.address;

    console.log("    ✅ Setup Complete");
  });

  it("Create Pool", async () => {
    const now = Math.floor(Date.now() / 1000);
    const START_TIME = new anchor.BN(now);
    const DURATION = 70;
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
        "Hybrid Test",
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

    console.log(`    ✅ Parimutuel Pool Created: ${POOL_NAME}`);
  });

  it("Users 1-5 Place Bets", async () => {
    const betAmount = new anchor.BN(100 * 1_000_000);

    for (let i = 0; i < users.length; i++) {
      if (i > 0) await sleep(200);

      const user = users[i];
      const commitment = createCommitment(predictions[i], salts[i]);

      const [betPda] = PublicKey.findProgramAddressSync(
        [
          SEED_BET,
          poolPda.toBuffer(),
          user.publicKey.toBuffer(),
          Buffer.from(requestIds[i]),
        ],
        program.programId
      );
      betPdas.push(betPda);

      await program.methods
        .placeBet(betAmount, Array.from(commitment), requestIds[i])
        .accountsPartial({
          user: user.publicKey,
          globalConfig: globalConfigPda,
          pool: poolPda,
          poolVault: vaultPda,
          userTokenAccount: userAtas[i],
          userBet: betPda,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([user])
        .rpc();
    }
    console.log("    ✅ All Bets Placed");
  });

  it("Delegate All, Update (User 1), Reveal All", async () => {
    // 1. Delegate
    console.log("    > Delegating...");
    for (let i = 0; i < users.length; i++) {
      await program.methods
        .delegateBet(requestIds[i])
        .accounts({
          user: users[i].publicKey,
          pool: poolPda,
          userBet: betPdas[i],
        })
        .signers([users[i]])
        .rpc();
      await sleep(50);
    }
    await sleep(5000);

    // 2. User 1 Updates Bet (Testing Single Update Logic)
    console.log("    > User 1 Updating (Single Op)...");
    const user1 = users[0];
    const erConnection = new anchor.web3.Connection(
      "https://devnet.magicblock.app"
    );
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
          userBet: betPdas[0],
          pool: null,
        })
        .rpc();
      console.log("    ✅ User 1 Updated");
    } catch (e) {
      console.error("    ❌ User 1 Update Fail:", e);
    }

    // 3. Reveal All (SKIP User 1 because Update reveals it automatically)
    console.log("    > Revealing...");
    for (let i = 0; i < users.length; i++) {
      if (i === 0) {
        console.log("      - Skipping User 1 (Already Revealed via Update)");
        continue;
      }

      const user = users[i];
      const erProvider = new anchor.AnchorProvider(
        erConnection,
        new anchor.Wallet(user),
        {}
      );
      const erProgram = new anchor.Program(program.idl, erProvider);

      try {
        await erProgram.methods
          .revealBet(predictions[i], Array.from(salts[i]))
          .accounts({ user: user.publicKey, userBet: betPdas[i] })
          .rpc();
        console.log(`      - User ${i + 1} Revealed`);
      } catch (e) {
        console.error(`      - User ${i + 1} Fail:`, e);
      }
      await sleep(200);
    }
  });

  it("Wait for Expiry", async () => {
    const now = Math.floor(Date.now() / 1000);
    const timeLeft = poolEndTime - now;
    if (timeLeft > 0) {
      console.log(
        `    ⏳ Waiting ${timeLeft + 5}s for expiry before undelegating...`
      );
      await sleep((timeLeft + 5) * 1000);
    }
  });

  // --- HYBRID UNDELEGATION ---
  it("Hybrid Undelegate (User 1 Single, Others Batch)", async () => {
    const erConnection = new anchor.web3.Connection(
      "https://devnet.magicblock.app"
    );

    // 1. SINGLE UNDELEGATE (User 1)
    console.log("\n    🛠️  User 1 Performing SINGLE UNDELEGATE...");
    const erProvider1 = new anchor.AnchorProvider(
      erConnection,
      new anchor.Wallet(users[0]),
      {}
    );
    const erProgram1 = new anchor.Program(program.idl, erProvider1);

    try {
      await performUndelegate(
        erProgram1,
        erConnection,
        users[0],
        requestIds[0],
        poolPda,
        betPdas[0]
      );
      console.log("    ✅ User 1 Single Undelegate Sent");
    } catch (e) {
      console.error("    ❌ User 1 Single Undelegate Failed", e);
    }

    // 2. BATCH UNDELEGATE (Users 2-5)
    console.log("\n    🛠️  Admin Performing BATCH UNDELEGATE (Users 2-5)...");
    const erProviderAdmin = new anchor.AnchorProvider(
      erConnection,
      new anchor.Wallet(admin),
      {}
    );
    const erProgramAdmin = new anchor.Program(program.idl, erProviderAdmin);

    const batchAccounts = betPdas.slice(1).map((pubkey) => ({
      pubkey,
      isWritable: true,
      isSigner: false,
    }));

    try {
      await erProgramAdmin.methods
        .batchUndelegateBets()
        .accounts({
          payer: admin.publicKey,
          pool: poolPda,
          magicProgram: MAGIC_PROGRAM_ID,
        })
        .remainingAccounts(batchAccounts)
        .rpc();

      console.log("    ✅ Batch Undelegate Transaction Sent");
    } catch (e) {
      console.error("    ❌ Batch Undelegate Failed:", e);
    }

    // 3. VERIFY OWNERSHIP
    await sleep(2000);
    console.log("    🔍 Verifying L1 Ownership...");
    for (let i = 0; i < users.length; i++) {
      const isBack = await waitForOwnership(
        provider.connection,
        betPdas[i],
        program.programId
      );
      if (isBack) console.log(`      ✅ User ${i + 1} confirmed on L1`);
    }
  });

  // --- HYBRID CALCULATION ---
  it("Resolve, Hybrid Calculate & Claim", async () => {
    // 1. RESOLVE POOL
    await program.methods
      .resolvePool(TARGET_PRICE)
      .accountsPartial({
        admin: admin.publicKey,
        globalConfig: globalConfigPda,
        pool: poolPda,
      })
      .rpc();
    console.log("    ✅ Pool Resolved");

    // 2. WAIT FOR BATCH DELAY (Rust requires > 5s after resolution)
    console.log("    ⏳ Waiting 6s for Batch Calculation Safety Period...");
    await sleep(6000);

    // 3. SINGLE CALCULATION (User 1)
    console.log("\n    🛠️  User 1 Performing SINGLE CALCULATION...");
    try {
      await program.methods
        .calculateOutcome()
        .accountsPartial({
          payer: users[0].publicKey,
          betOwner: users[0].publicKey,
          pool: poolPda,
          userBet: betPdas[0],
        })
        .signers([users[0]])
        .rpc();
      console.log("    ✅ User 1 Calculated");
    } catch (e) {
      console.error("    ❌ User 1 Calc Failed", e);
    }

    // 4. BATCH CALCULATE (Users 2-5)
    console.log("\n    🛠️  Admin Performing BATCH CALCULATION (Users 2-5)...");
    const batchAccounts = betPdas.slice(1).map((pubkey) => ({
      pubkey,
      isWritable: true,
      isSigner: false,
    }));

    try {
      await program.methods
        .batchCalculateOutcome()
        .accountsPartial({
          admin: admin.publicKey,
          pool: poolPda,
        })
        .remainingAccounts(batchAccounts)
        .rpc();
      console.log("    ✅ Batch Calculation Complete");
    } catch (e) {
      console.error("    ❌ Batch Calculation Failed:", e);
    }

    // 5. FINALIZE (Standard)
    const config = await program.account.globalConfig.fetch(globalConfigPda);
    const treasuryAtaInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      config.treasuryWallet,
      true
    );
    treasuryAta = treasuryAtaInfo.address;

    await program.methods
      .finalizeWeights()
      .accountsPartial({
        admin: admin.publicKey,
        globalConfig: globalConfigPda,
        pool: poolPda,
        poolVault: vaultPda,
        treasuryWallet: treasuryAta,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log("    ✅ Weights Finalized");

    // 6. CLAIM
    console.log("\n    📊 --- POOL RESULTS ---");
    for (let i = 0; i < users.length; i++) {
      const preBal = (
        await provider.connection.getTokenAccountBalance(userAtas[i])
      ).value.uiAmount;

      try {
        await program.methods
          .claimReward()
          .accountsPartial({
            user: users[i].publicKey,
            pool: poolPda,
            poolVault: vaultPda,
            userBet: betPdas[i],
            userTokenAccount: userAtas[i],
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([users[i]])
          .rpc();
      } catch (e) {}

      const postBal = (
        await provider.connection.getTokenAccountBalance(userAtas[i])
      ).value.uiAmount;
      console.log(
        `    User ${i + 1}: Payout ${(postBal - preBal).toFixed(2)} USDC`
      );
    }
  });
});
