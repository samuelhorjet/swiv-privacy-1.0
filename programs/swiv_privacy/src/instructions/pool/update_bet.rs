use anchor_lang::prelude::*;
use crate::state::{Bet, Pool, BetStatus};
use crate::constants::{SEED_POOL};
use crate::errors::CustomError;
use crate::events::BetUpdated;

#[derive(Accounts)]
pub struct UpdateBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = bet.user_pubkey == user.key() @ CustomError::Unauthorized,
        constraint = bet.status == BetStatus::Active @ CustomError::AlreadyClaimed
    )]
    pub bet: Box<Account<'info, Bet>>,

    #[account(
        seeds = [SEED_POOL, pool.created_by.as_ref(), &(pool.pool_id.to_le_bytes())],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,
}

pub fn update_bet(
    ctx: Context<UpdateBet>,
    new_prediction: u64, 
) -> Result<()> {
    let bet = &mut ctx.accounts.bet;
    let pool = &ctx.accounts.pool; 

    bet.update_count = bet.update_count.checked_add(1).unwrap();
    bet.prediction = new_prediction;
    
    msg!("Bet Updated securely via TEE. New prediction stored: {}", new_prediction);

    emit!(BetUpdated {
        bet_address: bet.key(),
        user: ctx.accounts.user.key(),
        pool_identifier: pool.title.clone(),
    });

    Ok(())
}