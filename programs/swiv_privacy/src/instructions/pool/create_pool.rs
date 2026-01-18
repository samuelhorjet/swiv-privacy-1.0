use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};
use crate::state::{Pool, GlobalConfig};
use crate::constants::{SEED_GLOBAL_CONFIG, SEED_POOL, SEED_POOL_VAULT};
use crate::errors::CustomError;
use crate::events::PoolCreated;

#[derive(Accounts)]
#[instruction(
    name: String, 
    metadata: Option<String>, 
    start_time: i64, 
    end_time: i64, 
    max_accuracy_buffer: u64,
    conviction_bonus_bps: u64 
)]
pub struct CreatePool<'info> {
    #[account(
        mut,
        seeds = [SEED_GLOBAL_CONFIG],
        bump,
        constraint = global_config.admin == admin.key() @ CustomError::Unauthorized
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        init,
        payer = admin,
        space = 200 + 8 + (4 + name.len()) + (4 + metadata.as_ref().map(|s| s.len()).unwrap_or(0)),
        seeds = [SEED_POOL, name.as_bytes()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        payer = admin,
        seeds = [SEED_POOL_VAULT, pool.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = pool,
    )]
    pub pool_vault: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, token::Mint>,

    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub admin_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_pool(
    ctx: Context<CreatePool>,
    name: String,
    metadata: Option<String>,
    start_time: i64,
    end_time: i64,
    max_accuracy_buffer: u64,
    conviction_bonus_bps: u64,
) -> Result<()> {
    require!(end_time > start_time, CustomError::DurationTooShort);
    
    let pool = &mut ctx.accounts.pool;
    pool.admin = ctx.accounts.admin.key();
    pool.name = name.clone();
    pool.metadata = metadata;
    pool.token_mint = ctx.accounts.token_mint.key();
    pool.start_time = start_time;
    pool.end_time = end_time;
    
    pool.vault_balance = 0; 
    pool.prize_pool = 0; 
    
    pool.max_accuracy_buffer = max_accuracy_buffer;
    pool.conviction_bonus_bps = conviction_bonus_bps; 
    
    pool.is_resolved = false;
    pool.resolution_target = 0;
    
    pool.total_weight = 0;
    pool.weight_finalized = false;

    pool.bump = ctx.bumps.pool;

    emit!(PoolCreated {
        pool_name: name,
        start_time,
        end_time,
    });

    Ok(())
}