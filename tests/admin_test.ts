import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SwivPrivacy } from "../target/types/swiv_privacy";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import {
  createMint,
} from "@solana/spl-token";
import { SEED_GLOBAL_CONFIG } from "./utils";

describe("1. Setup & Admin", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.SwivPrivacy as Program<SwivPrivacy>;
  const admin = (provider.wallet as anchor.Wallet).payer;

  it("Global Protocol Initialization", async () => {
    const usdcMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );
    const [configPda] = PublicKey.findProgramAddressSync(
      [SEED_GLOBAL_CONFIG],
      program.programId
    );

    try {
      await program.methods
        .initializeProtocol(new anchor.BN(300))
        .accountsPartial({
          admin: admin.publicKey,
          treasuryWallet: admin.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      console.log("    ✅ Protocol Initialized");
    } catch (e) {
      await program.methods
        .updateConfig(null, new anchor.BN(300))
        .accountsPartial({
          admin: admin.publicKey,
          globalConfig: configPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      console.log("    ✅ Protocol Config Updated");
    }
  });
});