import * as anchor from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair,
} from "@solana/web3.js";
import jsSha3 from "js-sha3";
const { keccak256 } = jsSha3;

import { MAGIC_PROGRAM_ID } from "@magicblock-labs/ephemeral-rollups-sdk";

// --- CONSTANTS ---
export const SEED_BET = Buffer.from("user_bet");
export const SEED_POOL = Buffer.from("pool"); 
export const SEED_GLOBAL_CONFIG = Buffer.from("global_config_v1");

// --- HELPERS ---
export const sleep = (ms: number) =>
  new Promise((resolve) => setTimeout(resolve, ms));

export function createCommitment(
  target: anchor.BN,
  salt: Buffer
) {
  const buf = Buffer.concat([
    target.toArrayLike(Buffer, "le", 8),
    salt,
  ]);
  return Buffer.from(keccak256.create().update(buf).arrayBuffer());
}

export async function retryOp<T>(
  operation: () => Promise<T>,
  description: string,
  maxRetries = 5,
  delayMs = 2000
): Promise<T> {
  let lastError: any;
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await operation();
    } catch (e: any) {
      console.log(
        `    ⚠️ ${description} failed (Attempt ${
          i + 1
        }/${maxRetries}). Retrying in ${delayMs}ms...`
      );
      lastError = e;
      await sleep(delayMs);
    }
  }
  console.error(
    `    ❌ ${description} failed permanently after ${maxRetries} attempts.`
  );
  throw lastError;
}

// --- ROBUST UNDELEGATE HELPER ---
export async function performUndelegate(
  erProgram: any,
  erConnection: anchor.web3.Connection,
  user: Keypair,
  requestId: string,
  poolPda: PublicKey, 
  betPda: PublicKey
) {
  let tx = await erProgram.methods
    .undelegateBet(requestId)
    .accounts({
      user: user.publicKey,
      pool: poolPda,
      userBet: betPda,
      magicProgram: MAGIC_PROGRAM_ID,
    })
    .transaction();

  tx.feePayer = user.publicKey;
  tx.recentBlockhash = (await erConnection.getLatestBlockhash()).blockhash;
  tx.sign(user);

  const sig = await erConnection.sendRawTransaction(tx.serialize(), {
    skipPreflight: true,
  });
  await erConnection.confirmTransaction(sig, "confirmed");
  return sig;
}