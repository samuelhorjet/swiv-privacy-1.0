use anchor_lang::prelude::*;
use crate::state::GlobalConfig;
use crate::constants::SEED_GLOBAL_CONFIG;
use crate::errors::CustomError;
use crate::events::ConfigUpdated;

#[derive(Accounts)]
#[instruction(
    new_treasury: Option<Pubkey>, 
    new_protocol_fee_bps: Option<u64> 
)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_GLOBAL_CONFIG],
        bump,
        constraint = global_config.admin == admin.key() @ CustomError::Unauthorized,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    pub system_program: Program<'info, System>,
}

pub fn update_config(
    ctx: Context<UpdateConfig>,
    new_treasury: Option<Pubkey>,
    new_protocol_fee_bps: Option<u64>,
) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;

    if let Some(treasury) = new_treasury {
        global_config.treasury_wallet = treasury;
    }

    if let Some(fee) = new_protocol_fee_bps {
        global_config.protocol_fee_bps = fee;
    }

    emit!(ConfigUpdated {
        treasury: new_treasury,
        protocol_fee_bps: new_protocol_fee_bps,
    });

    msg!("Global Config Updated");

    Ok(())
}