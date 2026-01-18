use anchor_lang::prelude::*;
use crate::state::{UserBet, Pool, BetStatus};
use crate::constants::{SEED_POOL};
use crate::errors::CustomError;
use crate::events::BetUpdated;

#[derive(Accounts)]
pub struct UpdateBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_bet.owner == user.key() @ CustomError::Unauthorized,
        constraint = user_bet.status == BetStatus::Active @ CustomError::AlreadySettled
    )]
    pub user_bet: Box<Account<'info, UserBet>>,

    #[account(
        seeds = [SEED_POOL, user_bet.pool_identifier.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,
    
}

pub fn update_bet(
    ctx: Context<UpdateBet>,
    new_prediction_target: u64, 
) -> Result<()> {
    let user_bet = &mut ctx.accounts.user_bet;
    let pool = &ctx.accounts.pool; 
    let clock = Clock::get()?;

    require!(clock.unix_timestamp < pool.end_time, CustomError::DurationTooShort);

    user_bet.creation_ts = clock.unix_timestamp;
    
    user_bet.update_count = user_bet.update_count.checked_add(1).unwrap();

    user_bet.prediction_target = new_prediction_target;
    
    user_bet.is_revealed = true; 
    
    msg!("Bet Updated securely via TEE. New Target: {}", new_prediction_target);

    emit!(BetUpdated {
        bet_address: user_bet.key(),
        user: ctx.accounts.user.key(),
    });

    Ok(())
}