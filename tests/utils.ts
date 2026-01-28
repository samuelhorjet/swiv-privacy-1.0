import { PublicKey } from "@solana/web3.js";

export const SEED_BET = Buffer.from("user_bet");
export const SEED_POOL = Buffer.from("pool");
export const SEED_POOL_VAULT = Buffer.from("pool_vault");
export const SEED_PROTOCOL = Buffer.from("protocol_v2");

export const TEE_VALIDATOR = new PublicKey("FnE6VJT5QNZdedZPnCoLsARgBwoE6DeJNjBs2H1gySXA");

export const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export { 
  permissionPdaFromAccount, 
  verifyTeeRpcIntegrity, 
  getAuthToken, 
  PERMISSION_PROGRAM_ID, 
  DELEGATION_PROGRAM_ID, 
  delegationRecordPdaFromDelegatedAccount, 
  delegationMetadataPdaFromDelegatedAccount, 
  delegateBufferPdaFromDelegatedAccountAndOwnerProgram 
} from "@magicblock-labs/ephemeral-rollups-sdk";