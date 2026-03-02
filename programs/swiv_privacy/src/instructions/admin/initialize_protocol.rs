use anchor_lang::prelude::*;
use crate::state::Protocol;
use crate::constants::{SEED_PROTOCOL};
use crate::events::ProtocolInitialized;

#[derive(Accounts)]
#[instruction(
    protocol_fee_bps: u64 
)]
pub struct InitializeProtocol<'info> {
    #[account(
        init,
        payer = admin,
        space = Protocol::BASE_LEN,
        seeds = [SEED_PROTOCOL],
        bump
    )]
    pub protocol: Account<'info, Protocol>,

    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: This is the wallet that receives fees. Safe to be any address.
    pub treasury_wallet: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_protocol(
    ctx: Context<InitializeProtocol>,
    protocol_fee_bps: u64 
) -> Result<()> {
    let protocol = &mut ctx.accounts.protocol;
    
    protocol.admin = ctx.accounts.admin.key();
    protocol.treasury_wallet = ctx.accounts.treasury_wallet.key();
    
    protocol.protocol_fee_bps = protocol_fee_bps;
    
    protocol.paused = false;
    protocol.total_pools = 0;
    protocol.batch_settle_wait_duration = 60; 

    emit!(ProtocolInitialized {
        admin: ctx.accounts.admin.key(),
        fee_wallet: ctx.accounts.treasury_wallet.key(),
    });

    Ok(())
}