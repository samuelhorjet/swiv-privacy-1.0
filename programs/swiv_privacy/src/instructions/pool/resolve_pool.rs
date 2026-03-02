use anchor_lang::prelude::*;
use crate::state::{Pool, Protocol};
use crate::constants::{SEED_PROTOCOL, SEED_POOL};
use crate::errors::CustomError;
use crate::events::PoolResolved;

#[derive(Accounts)]
pub struct ResolvePool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [SEED_PROTOCOL],
        bump,
        constraint = protocol.admin == admin.key() @ CustomError::Unauthorized
    )]
    pub protocol: Account<'info, Protocol>,

    #[account(
        mut,
        seeds = [SEED_POOL, pool.created_by.as_ref(), &(pool.pool_id.to_le_bytes())],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,
}

pub fn resolve_pool(ctx: Context<ResolvePool>, final_outcome: u64) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    
    require!(!pool.is_resolved, CustomError::AlreadyClaimed);
    
    let clock = Clock::get()?;
    require!(clock.unix_timestamp >= pool.end_time, CustomError::DurationTooShort);

    pool.resolution_result = final_outcome;
    pool.is_resolved = true;
    
    pool.resolution_ts = clock.unix_timestamp; 
    pool.weight_finalized = false; 
    
    emit!(PoolResolved {
        pool_name: pool.title.clone(),
        final_outcome,
        resolution_ts: pool.resolution_ts,
    });

    msg!("Pool Resolved. Outcome: {}", final_outcome);
    
    Ok(())
}