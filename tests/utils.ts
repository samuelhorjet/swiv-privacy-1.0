import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

// Constants matching your Rust constants.rs
export const SEED_BET = Buffer.from("user_bet");
export const SEED_POOL = Buffer.from("pool");
export const SEED_POOL_VAULT = Buffer.from("pool_vault");
export const SEED_GLOBAL_CONFIG = Buffer.from("global_config_v1");

export const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

import { permissionPdaFromAccount, verifyTeeRpcIntegrity, getAuthToken, PERMISSION_PROGRAM_ID } from "@magicblock-labs/ephemeral-rollups-sdk";

export { permissionPdaFromAccount, verifyTeeRpcIntegrity, getAuthToken, PERMISSION_PROGRAM_ID };

export function getPermissionPda(accountToLock: PublicKey): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("permission"), accountToLock.toBuffer()],
    PERMISSION_PROGRAM_ID
  );
  return pda;
}

export function getGroupPda(accountToLock: PublicKey): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("group"), accountToLock.toBuffer()],
    PERMISSION_PROGRAM_ID
  );
  return pda;
}