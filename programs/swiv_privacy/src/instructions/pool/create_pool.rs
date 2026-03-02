use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};
use crate::state::{Pool, Protocol};
use crate::constants::{SEED_PROTOCOL, SEED_POOL, SEED_POOL_VAULT}; 
use crate::errors::CustomError;
use crate::events::PoolCreated;

#[derive(Accounts)]
#[instruction(
    pool_id: u64,
    title: String, 
    start_time: i64, 
    end_time: i64, 
    max_accuracy_buffer: u64,
    conviction_bonus_bps: u64
)]
pub struct CreatePool<'info> {
    #[account(
        mut,
        seeds = [SEED_PROTOCOL],
        bump,
        constraint = protocol.admin == created_by.key() @ CustomError::Unauthorized
    )]
    pub protocol: Account<'info, Protocol>,

    #[account(
        init,
        payer = created_by,
        space = 8 + 300, 
        seeds = [SEED_POOL, created_by.key().as_ref(), &pool_id.to_le_bytes()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        payer = created_by,
        seeds = [SEED_POOL_VAULT, pool.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = pool,
    )]
    pub pool_vault: Account<'info, TokenAccount>,
    
    pub token_mint: Account<'info, token::Mint>,

    #[account(mut)]
    pub created_by: Signer<'info>,

    #[account(mut)]
    pub created_by_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_pool(
    ctx: Context<CreatePool>,
    pool_id: u64,
    title: String,
    start_time: i64,
    end_time: i64,
    max_accuracy_buffer: u64,
    conviction_bonus_bps: u64,
) -> Result<()> {
    require!(end_time > start_time, CustomError::DurationTooShort);

    let pool = &mut ctx.accounts.pool;
    let protocol = &mut ctx.accounts.protocol;
    
    pool.created_by = ctx.accounts.created_by.key();
    pool.title = title.clone();
    pool.pool_id = pool_id;
    pool.stake_token_mint = ctx.accounts.token_mint.key();
    pool.start_time = start_time;
    pool.end_time = end_time;
    pool.total_volume = 0;
    pool.total_participants = 0;
    pool.max_accuracy_buffer = max_accuracy_buffer;
    pool.conviction_bonus_bps = conviction_bonus_bps; 
    
    pool.is_resolved = false;
    pool.resolution_result = 0;
    
    pool.total_weight = 0;
    pool.weight_finalized = false;
    pool.bump = ctx.bumps.pool;
    
    protocol.total_pools = protocol.total_pools.checked_add(1).unwrap();
    
    emit!(PoolCreated {
        pool_name: title,
        start_time,
        end_time,
    });

    Ok(())
}