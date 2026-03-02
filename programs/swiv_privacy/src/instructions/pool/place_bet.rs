use anchor_lang::prelude::*;
use crate::state::{Pool, Bet, BetStatus};
use crate::constants::{SEED_BET, SEED_POOL}; 
use crate::errors::CustomError;
use crate::events::BetPlaced;

#[derive(Accounts)]
#[instruction(prediction: u64, request_id: String)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [SEED_POOL, pool.created_by.as_ref(), &(pool.pool_id.to_le_bytes())],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        seeds = [SEED_BET, pool.key().as_ref(), user.key().as_ref(), request_id.as_bytes()], 
        bump = bet.bump,
        constraint = bet.user_pubkey == user.key() @ CustomError::Unauthorized
    )]
    pub bet: Box<Account<'info, Bet>>,
}

pub fn place_bet(
    ctx: Context<PlaceBet>,
    prediction: u64, 
    _request_id: String, 
) -> Result<()> {
    let bet = &mut ctx.accounts.bet;
    let pool = &ctx.accounts.pool;

    require!(bet.status == BetStatus::Pending, CustomError::BetAlreadyInitialized);

    bet.prediction = prediction;
    bet.status = BetStatus::Active;
    bet.update_count = bet.update_count.checked_add(1).unwrap();

    emit!(BetPlaced {
        bet_address: bet.key(),
        user: ctx.accounts.user.key(),
        pool_identifier: pool.title.clone(),
        amount: bet.stake,
        end_timestamp: pool.end_time,
    });

    Ok(())
}