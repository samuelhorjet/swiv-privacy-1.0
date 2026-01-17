use anchor_lang::prelude::*;
use crate::state::GlobalConfig;
use crate::constants::SEED_GLOBAL_CONFIG;
use crate::errors::CustomError;
use crate::events::AdminTransferred;

#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    #[account(mut)]
    pub current_admin: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_GLOBAL_CONFIG],
        bump,
        constraint = global_config.admin == current_admin.key() @ CustomError::Unauthorized
    )]
    pub global_config: Account<'info, GlobalConfig>,
}

pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let old_admin = global_config.admin;
    
    global_config.admin = new_admin;
    
    emit!(AdminTransferred {
        old_admin,
        new_admin,
    });

    msg!("Admin transferred from {} to {}", old_admin, new_admin);

    Ok(())
}