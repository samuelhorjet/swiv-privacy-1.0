use anchor_lang::prelude::*;
use crate::state::{Pool, UserBet, BetStatus};
use crate::constants::{SEED_BET, SEED_POOL}; 
use crate::errors::CustomError;
use crate::events::BetPlaced;

#[derive(Accounts)]
#[instruction(prediction: u64, request_id: String)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [SEED_POOL, pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        seeds = [SEED_BET, pool.key().as_ref(), user.key().as_ref(), request_id.as_bytes()], 
        bump = user_bet.bump,
        constraint = user_bet.owner == user.key() @ CustomError::Unauthorized
    )]
    pub user_bet: Box<Account<'info, UserBet>>,
}

pub fn place_bet(
    ctx: Context<PlaceBet>,
    prediction: u64, 
    _request_id: String, 
) -> Result<()> {
    let user_bet = &mut ctx.accounts.user_bet;
    let pool = &ctx.accounts.pool;
    let clock = Clock::get()?;

    require!(user_bet.status == BetStatus::Initialized, CustomError::BetAlreadyInitialized);

    require!(clock.unix_timestamp < pool.end_time, CustomError::DurationTooShort); 

    user_bet.prediction = prediction;
    user_bet.status = BetStatus::Active;
    user_bet.update_count = user_bet.update_count.checked_add(1).unwrap();

    emit!(BetPlaced {
        bet_address: user_bet.key(),
        user: ctx.accounts.user.key(),
        pool_identifier: pool.name.clone(),
        amount: user_bet.deposit,
        end_timestamp: pool.end_time,
    });

    Ok(())
}