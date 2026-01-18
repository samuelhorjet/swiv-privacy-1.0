use anchor_lang::prelude::*;
use crate::state::{Pool, UserBet, BetStatus};
use crate::constants::{SEED_POOL};
use crate::errors::CustomError;
use crate::events::OutcomeCalculated;
use crate::utils::math::{
    calculate_accuracy_score, 
    calculate_time_bonus, 
    calculate_conviction_bonus, 
    calculate_weight, 
};

#[derive(Accounts)]
pub struct CalculateOutcome<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: We verify this matches user_bet.owner below
    pub bet_owner: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [SEED_POOL, pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        constraint = user_bet.owner == bet_owner.key() @ CustomError::Unauthorized,
        constraint = user_bet.status == BetStatus::Active @ CustomError::AlreadySettled,
        constraint = user_bet.is_revealed @ CustomError::BetNotRevealed
    )]
    pub user_bet: Account<'info, UserBet>,
}

pub fn calculate_outcome(ctx: Context<CalculateOutcome>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let bet = &mut ctx.accounts.user_bet;

    require!(pool.is_resolved, CustomError::SettlementTooEarly);
    require!(!bet.is_weight_added, CustomError::AlreadySettled);

    let user_prediction = bet.prediction_target;
    let result = pool.resolution_target;

    let accuracy_score = calculate_accuracy_score(
        user_prediction,
        result,
        pool.max_accuracy_buffer
    )?;

    let time_bonus = calculate_time_bonus(
        pool.start_time,
        pool.end_time,
        bet.creation_ts
    )?;

    let conviction_bonus = calculate_conviction_bonus(bet.update_count);

    let weight = calculate_weight(
        bet.deposit,
        accuracy_score,
        time_bonus,
        conviction_bonus
    )?;

    pool.total_weight = pool.total_weight.checked_add(weight).unwrap();
    
    bet.calculated_weight = weight;
    bet.is_weight_added = true;
    bet.status = BetStatus::Calculated;

    emit!(OutcomeCalculated {
        bet_address: bet.key(),
        user: ctx.accounts.bet_owner.key(),
        weight: weight,
    });

    msg!("Calculated Weight for User {}: {}", ctx.accounts.bet_owner.key(), weight);

    Ok(())
}