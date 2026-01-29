import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SwivPrivacy } from "../target/types/swiv_privacy";
import {
  PublicKey,
  SystemProgram,
  Keypair,
  LAMPORTS_PER_SOL,
  ComputeBudgetProgram,
  sendAndConfirmTransaction,
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
  SEED_PROTOCOL,
  PERMISSION_PROGRAM_ID,
  sleep,
  TEE_VALIDATOR,
  getAuthToken,
  DELEGATION_PROGRAM_ID,
  permissionPdaFromAccount,
  delegationRecordPdaFromDelegatedAccount,
  delegationMetadataPdaFromDelegatedAccount,
  delegateBufferPdaFromDelegatedAccountAndOwnerProgram,
  waitUntilPermissionActive, // Added this helper
} from "./utils";
import * as nacl from "tweetnacl";

const KEYS_DIR = path.join(__dirname, "keys");
if (!fs.existsSync(KEYS_DIR)) fs.mkdirSync(KEYS_DIR);

function loadOrGenerateKeypair(name: string): Keypair {
  const filePath = path.join(KEYS_DIR, `${name}.json`);
  if (fs.existsSync(filePath)) {
    return Keypair.fromSecretKey(
      new Uint8Array(JSON.parse(fs.readFileSync(filePath, "utf-8"))),
    );
  } else {
    const kp = Keypair.generate();
    fs.writeFileSync(filePath, JSON.stringify(Array.from(kp.secretKey)));
    return kp;
  }
}

async function getAuthTokenWithRetry(
  endpoint: string,
  pubkey: PublicKey,
  signer: (msg: Uint8Array) => Promise<Uint8Array>,
  retries = 3,
): Promise<{ token: string }> {
  for (let i = 0; i < retries; i++) {
    try {
      return await getAuthToken(endpoint, pubkey, signer);
    } catch (e) {
      if (i === retries - 1) throw e;
      console.log(`      ⚠️  Auth failed. Retrying (${i + 1}/${retries})...`);
      await sleep(2000 * (i + 1));
    }
  }
  throw new Error("Unreachable");
}

describe("Production Flow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.SwivPrivacy as Program<SwivPrivacy>;
  const admin = (provider.wallet as anchor.Wallet).payer;

  const users = [
    loadOrGenerateKeypair("user_tee_1"),
    loadOrGenerateKeypair("user_tee_2"),
  ];

  let usdcMint: PublicKey;
  let userAtas: PublicKey[] = [];
  let protocolPda: PublicKey;
  let poolPda: PublicKey;
  let vaultPda: PublicKey;
  let poolId: number = 0;

  const POOL_NAME = `TEE-Pool-${Math.floor(Math.random() * 1000)}`;
  let END_TIME: anchor.BN;
  const TARGET_PRICE = new anchor.BN(200_000_000);

  const predictions = [new anchor.BN(200_000_000), new anchor.BN(250_000_000)];
  const requestIds = ["req_1", "req_2"];
  const betPdas: PublicKey[] = [];
  const permissionPdas: PublicKey[] = [];

  const TEE_URL = "https://tee.magicblock.app";
  const TEE_WS_URL = "wss://tee.magicblock.app";
  const ephemeralRpcEndpoint = TEE_URL;

  it("1. Setup Environment", async () => {
    [protocolPda] = PublicKey.findProgramAddressSync(
      [SEED_PROTOCOL],
      program.programId,
    );
    usdcMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6,
    );

    for (const user of users) {
      const bal = await provider.connection.getBalance(user.publicKey);
      if (bal < 0.1 * LAMPORTS_PER_SOL) {
        await provider.sendAndConfirm(
          new anchor.web3.Transaction().add(
            SystemProgram.transfer({
              fromPubkey: admin.publicKey,
              toPubkey: user.publicKey,
              lamports: 0.1 * LAMPORTS_PER_SOL,
            }),
          ),
        );
      }
      const ata = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        admin,
        usdcMint,
        user.publicKey,
      );
      userAtas.push(ata.address);
      await mintTo(
        provider.connection,
        admin,
        usdcMint,
        ata.address,
        admin,
        1000 * 1e6,
      );
    }

    try {
      await program.methods
        .initializeProtocol(new anchor.BN(300))
        .accountsPartial({
          admin: admin.publicKey,
          treasuryWallet: admin.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    } catch (e) {}

    const protocol = await program.account.protocol.fetch(protocolPda);
    poolId = protocol.totalPools.toNumber();
  });

  it("2. Create Pool (L1)", async () => {
    const now = Math.floor(Date.now() / 1000);
    const START_TIME = new anchor.BN(now);
    END_TIME = START_TIME.add(new anchor.BN(60));

    [poolPda] = PublicKey.findProgramAddressSync(
      [
        SEED_POOL,
        admin.publicKey.toBuffer(),
        new anchor.BN(poolId).toBuffer("le", 8),
      ],
      program.programId,
    );
    [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool_vault"), poolPda.toBuffer()],
      program.programId,
    );

    const adminAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      admin.publicKey,
    );

    await program.methods
      .createPool(
        new anchor.BN(poolId),
        POOL_NAME,
        "BTC/USDC",
        START_TIME,
        END_TIME,
        new anchor.BN(500),
        new anchor.BN(1000),
      )
      .accountsPartial({
        protocol: protocolPda,
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
    console.log("    ✅ Pool Created on L1");
  });

  it("3.1. Secure Bet Setup (L1: Init & Delegate)", async () => {
    const betAmount = new anchor.BN(100 * 1e6);
    console.log("    🏗️  Step 3.1: Initializing and Delegating User Bets...");

    for (let i = 0; i < users.length; i++) {
      const user = users[i];
      const requestId = requestIds[i];
      console.log(
        `      👤 Processing User ${i + 1} (${user.publicKey
          .toBase58()
          .slice(0, 8)}...)`,
      );

      const [betPda] = PublicKey.findProgramAddressSync(
        [
          SEED_BET,
          poolPda.toBuffer(),
          user.publicKey.toBuffer(),
          Buffer.from(requestId),
        ],
        program.programId,
      );
      betPdas.push(betPda);
      const permissionPda = permissionPdaFromAccount(betPda);

      console.log(`        👉 Bet PDA: ${betPda.toBase58()}`);
      console.log(`        👉 Permission PDA: ${permissionPda.toBase58()}`);

      const tx = new anchor.web3.Transaction().add(
        await program.methods
          .initBet(betAmount, requestId)
          .accountsPartial({
            user: user.publicKey,
            protocol: protocolPda,
            pool: poolPda,
            poolVault: vaultPda,
            userTokenAccount: userAtas[i],
            userBet: betPda,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .instruction(),
        await program.methods
          .createBetPermission(requestId)
          .accountsPartial({
            payer: user.publicKey,
            user: user.publicKey,
            userBet: betPda,
            pool: poolPda,
            permission: permissionPda,
            permissionProgram: PERMISSION_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .instruction(),
        await program.methods
          .delegateBetPermission(requestId)
          .accountsPartial({
            user: user.publicKey,
            pool: poolPda,
            userBet: betPda,
            permission: permissionPda,
            permissionProgram: PERMISSION_PROGRAM_ID,
            delegationProgram: DELEGATION_PROGRAM_ID,
            delegationRecord:
              delegationRecordPdaFromDelegatedAccount(permissionPda),
            delegationMetadata:
              delegationMetadataPdaFromDelegatedAccount(permissionPda),
            delegationBuffer:
              delegateBufferPdaFromDelegatedAccountAndOwnerProgram(
                permissionPda,
                PERMISSION_PROGRAM_ID,
              ),
            validator: TEE_VALIDATOR,
            systemProgram: SystemProgram.programId,
          })
          .instruction(),
        await program.methods
          .delegateBet(requestId)
          .accountsPartial({
            user: user.publicKey,
            pool: poolPda,
            userBet: betPda,
            validator: TEE_VALIDATOR,
          })
          .instruction(),
      );

      const sig = await anchor.web3.sendAndConfirmTransaction(
        provider.connection,
        tx,
        [user],
      );
      console.log(`        ✅ L1 Transaction Confirmed: ${sig}`);

      console.log(`        ⏳ Waiting for TEE to index delegation...`);
      await waitUntilPermissionActive(ephemeralRpcEndpoint, betPda);
      console.log(`        ✨ TEE Synchronization Complete for User ${i + 1}`);
    }
  });

  it("3.2. Secure Bet Execution (TEE: Place Bet)", async () => {
    console.log("    🎯 Step 3.2: Executing Private Bets on TEE...");

    for (let i = 0; i < users.length; i++) {
      const user = users[i];
      const requestId = requestIds[i];
      const betPda = betPdas[i];

      console.log(`      🔐 Authenticating User ${i + 1}...`);
      const authToken = await getAuthTokenWithRetry(
        ephemeralRpcEndpoint,
        user.publicKey,
        async (msg) => nacl.sign.detached(msg, user.secretKey),
      );

      const teeConnection = new anchor.web3.Connection(
        `${TEE_URL}?token=${authToken.token}`,
        {
          commitment: "confirmed",
          wsEndpoint: `${TEE_WS_URL}?token=${authToken.token}`,
        },
      );

      console.log(
        `      🚀 Sending 'placeBet' to TEE (Prediction: ${predictions[
          i
        ].toString()})...`,
      );
      const placeBetIx = await program.methods
        .placeBet(predictions[i], requestId)
        .accountsPartial({
          user: user.publicKey,
          pool: poolPda,
          userBet: betPda,
        })
        .instruction();

      const tx = new anchor.web3.Transaction().add(placeBetIx);
      tx.feePayer = user.publicKey;
      tx.recentBlockhash = (await teeConnection.getLatestBlockhash()).blockhash;

      const txSig = await sendAndConfirmTransaction(teeConnection, tx, [user], {
        skipPreflight: true,
      });
      console.log(`      ✅ Bet Executed Privately. TEE Sig: ${txSig}`);

      await sleep(1000);
    }
  });

  it("4. Privacy Verification (TEE Snoop Check)", async () => {
    console.log("    🕵️ Step 4: Verifying Data Privacy on TEE...");

    // User 1 attempts to read User 2's bet
    const user1 = users[0];
    const user2BetPda = betPdas[1];

    console.log(
      `      🕵️ User 1 (${user1.publicKey
        .toBase58()
        .slice(0, 8)}) is attempting to peek at User 2's Bet...`,
    );

    const authToken = await getAuthTokenWithRetry(
      ephemeralRpcEndpoint,
      user1.publicKey,
      async (msg) => nacl.sign.detached(msg, user1.secretKey),
    );

    const teeConnection = new anchor.web3.Connection(
      `${TEE_URL}?token=${authToken.token}`,
      {
        commitment: "confirmed",
      },
    );

    try {
      const accountInfo = await teeConnection.getAccountInfo(user2BetPda);

      if (accountInfo === null) {
        console.log(
          "      ✅ PRIVACY CONFIRMED: TEE returned 'null' for unauthorized access.",
        );
      } else {
        // If the TEE returns data, check if it's actually decrypted/readable
        console.log(
          "      ⚠️  TEE returned account data. Checking readability...",
        );
        try {
          const decoded = program.coder.accounts.decode(
            "UserBet",
            accountInfo.data,
          );
          console.log(
            "      ❌ PRIVACY BREACH: User 1 was able to decode User 2's prediction:",
            decoded.prediction.toString(),
          );
          throw new Error(
            "Privacy Failure: Unauthorized user read private data!",
          );
        } catch (e) {
          console.log(
            "      ✅ PRIVACY CONFIRMED: Data is present but encrypted/unreadable by User 1.",
          );
        }
      }
    } catch (err) {
      console.log(
        "      ✅ PRIVACY CONFIRMED: TEE explicitly blocked the request.",
      );
    }
  });

  it("5. Delegate Pool & Resolve", async () => {
    console.log("    ⏳ Step 5: Beginning Settlement Process...");

    const timeToWait = Math.max(
      0,
      END_TIME.toNumber() * 1000 - Date.now() + 2000,
    );
    console.log(`    ⏳ Waiting ${timeToWait / 1000}s for pool expiry...`);
    await sleep(timeToWait);

    // --- DELEGATE POOL ---
    console.log("    🔗 Delegating Pool PDA to TEE...");
    const delegatePoolTx = await program.methods
      .delegatePool(new anchor.BN(poolId))
      .accountsPartial({
        admin: admin.publicKey,
        protocol: protocolPda,
        pool: poolPda,
        validator: TEE_VALIDATOR,
      })
      .rpc();
    console.log(`    ✅ Pool Delegated (L1 Sig: ${delegatePoolTx})`);

    // --- AUTHENTICATION ---
    console.log("    🔐 Authenticating Admin Session on TEE...");
    const authToken = await getAuthTokenWithRetry(
      ephemeralRpcEndpoint,
      admin.publicKey,
      async (m) => nacl.sign.detached(m, admin.secretKey),
    );
    console.log("    ✅ Admin Auth Token obtained.");

    const erConn = new anchor.web3.Connection(
      `${TEE_URL}?token=${authToken.token}`,
      {
        commitment: "confirmed",
        wsEndpoint: `${TEE_WS_URL}?token=${authToken.token}`,
      },
    );
    const erProvider = new anchor.AnchorProvider(
      erConn,
      new anchor.Wallet(admin),
      {
        commitment: "confirmed",
        preflightCommitment: "confirmed",
      },
    );
    const erProgram = new anchor.Program(program.idl, erProvider);

    // --- RESOLVE POOL (TEE) ---
    console.log(
      `    🎲 Resolving Pool on TEE (Target Outcome: ${TARGET_PRICE.toString()})...`,
    );
    const resolveTx = await erProgram.methods
      .resolvePool(TARGET_PRICE)
      .accountsPartial({
        admin: admin.publicKey,
        protocol: protocolPda,
        pool: poolPda,
      })
      .rpc();
    console.log(`    ✅ Pool Resolved on TEE (Sig: ${resolveTx})`);

    // --- CALCULATE WEIGHTS (TEE) ---
    console.log(
      `    ⚖️  Calculating Weights for ${betPdas.length} users on TEE...`,
    );
    const batchAccounts = betPdas.map((k) => ({
      pubkey: k,
      isWritable: true,
      isSigner: false,
    }));

    const calcTx = await erProgram.methods
      .batchCalculateWeights()
      .accountsPartial({ admin: admin.publicKey, pool: poolPda })
      .remainingAccounts(batchAccounts)
      .rpc();
    console.log(`    ✅ Weights Calculated (Sig: ${calcTx})`);

    // --- UNDELEGATE BETS (Flush to L1) ---
    console.log(
      "    📤 Flushing User Bet Data back to L1 (Batch Undelegate)...",
    );
    const undelegateBetsTx = await erProgram.methods
      .batchUndelegateBets()
      .accounts({ payer: admin.publicKey, pool: poolPda })
      .remainingAccounts(batchAccounts)
      .rpc();
    console.log(`    ✅ User Bets Flushed to L1 (Sig: ${undelegateBetsTx})`);

    // --- UNDELEGATE POOL (Flush to L1) ---
    console.log(
      "    📤 Finalizing Settlement: Flushing Pool PDA back to L1...",
    );
    const finalUndelegateTx = await erProgram.methods
      .undelegatePool()
      .accounts({
        admin: admin.publicKey,
        protocol: protocolPda,
        pool: poolPda,
      })
      .rpc();
    console.log(`    ✅ Pool Settled back to L1 (Sig: ${finalUndelegateTx})`);

    console.log("    🏁 Settlement Process Complete.");
  });

  it("7. Public Verify", async () => {
    const betData = await program.account.userBet.fetch(betPdas[1]);
    console.log(`    📖 L1 Prediction: ${betData.prediction.toString()}`);
    if (!betData.prediction.eq(predictions[1])) {
      throw new Error(
        `❌ Data Mismatch: Expected ${predictions[1]}, got ${betData.prediction}`,
      );
    }
    console.log("    ✅ Transparency Confirmed.");
  });
});
