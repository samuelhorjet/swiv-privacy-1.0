use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{Pool, GlobalConfig};
use crate::constants::{SEED_GLOBAL_CONFIG, SEED_POOL, SEED_POOL_VAULT};
use crate::errors::CustomError;
use crate::events::WeightsFinalized;

#[derive(Accounts)]
pub struct FinalizeWeights<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [SEED_GLOBAL_CONFIG],
        bump,
        constraint = global_config.admin == admin.key() @ CustomError::Unauthorized
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        seeds = [SEED_POOL, pool.name.as_bytes()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [SEED_POOL_VAULT, pool.key().as_ref()],
        bump,
        token::authority = pool,
    )]
    pub pool_vault: Account<'info, TokenAccount>,

    /// CHECK: Validated against GlobalConfig
    #[account(
        mut, 
        token::authority = global_config.treasury_wallet
    )]
    pub treasury_wallet: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn finalize_weights(ctx: Context<FinalizeWeights>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let global_config = &ctx.accounts.global_config;
    
    require!(pool.is_resolved, CustomError::SettlementTooEarly);
    require!(!pool.weight_finalized, CustomError::AlreadySettled);

    let total_pot = pool.vault_balance;
    let fee_amount = total_pot
        .checked_mul(global_config.protocol_fee_bps).unwrap()
        .checked_div(10000).unwrap();

    if fee_amount > 0 {
        let name_bytes = pool.name.as_bytes();
        let bump = pool.bump;
        let seeds = &[SEED_POOL, name_bytes, &[bump]];
        let signer = &[&seeds[..]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_vault.to_account_info(),
                    to: ctx.accounts.treasury_wallet.to_account_info(),
                    authority: pool.to_account_info(),
                },
                signer,
            ),
            fee_amount,
        )?;

        pool.vault_balance = pool.vault_balance.checked_sub(fee_amount).unwrap();
        msg!("Protocol Fee Deducted: {}", fee_amount);
    }

    pool.weight_finalized = true;
    
    emit!(WeightsFinalized {
        pool_name: pool.name.clone(),
        total_weight: pool.total_weight,
        fee_deducted: fee_amount,
    });
    
    msg!("Weights Finalized. Total Distributable: {}, Total Weight: {}", pool.vault_balance, pool.total_weight);

    Ok(())
}