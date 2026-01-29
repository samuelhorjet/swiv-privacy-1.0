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
        constraint = user_bet.status == BetStatus::Active @ CustomError::AlreadyClaimed
    )]
    pub user_bet: Box<Account<'info, UserBet>>,

    #[account(
        seeds = [SEED_POOL, pool.admin.as_ref(), &(pool.pool_id.to_le_bytes())],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,
}

pub fn update_bet(
    ctx: Context<UpdateBet>,
    new_prediction: u64, 
) -> Result<()> {
    let user_bet = &mut ctx.accounts.user_bet;
    let pool = &ctx.accounts.pool; 

    user_bet.update_count = user_bet.update_count.checked_add(1).unwrap();
    user_bet.prediction = new_prediction;
    
    msg!("Bet Updated securely via TEE. New prediction stored: {}", new_prediction);

    emit!(BetUpdated {
        bet_address: user_bet.key(),
        user: ctx.accounts.user.key(),
        pool_identifier: pool.name.clone(),
    });

    Ok(())
}