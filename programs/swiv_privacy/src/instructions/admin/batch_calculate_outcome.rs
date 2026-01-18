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
pub struct BatchCalculateOutcome<'info> {
    #[account(mut)]
    pub admin: Signer<'info>, 

    #[account(
        mut,
        seeds = [SEED_POOL, pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,
}

pub fn batch_calculate_outcome<'info>(
    ctx: Context<'_, '_, '_, 'info, BatchCalculateOutcome<'info>>
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let accounts_iter = &mut ctx.remaining_accounts.iter();
    let clock = Clock::get()?;

    require!(pool.is_resolved, CustomError::SettlementTooEarly);
    require!(!pool.weight_finalized, CustomError::AlreadySettled);

    let batch_wait_duration = 5; 
    
    require!(
        clock.unix_timestamp > pool.resolution_ts + batch_wait_duration,
        CustomError::SettlementTooEarly
    );

    let result = pool.resolution_target;
    let start_time = pool.start_time;
    let end_time = pool.end_time;
    let max_accuracy_buffer = pool.max_accuracy_buffer;

    loop {
        let user_bet_acc_info = match accounts_iter.next() {
            Some(acc) => acc,
            None => break,
        };

        let mut user_bet_data = user_bet_acc_info.try_borrow_mut_data()?;
        let mut user_bet = UserBet::try_deserialize(&mut &user_bet_data[..])?;

        if user_bet.pool_identifier != pool.name { continue; }
        if user_bet.status != BetStatus::Active || !user_bet.is_revealed { continue; }

        let accuracy_score = calculate_accuracy_score(
            user_bet.prediction_target,
            result,
            max_accuracy_buffer
        )?;

        let time_bonus = calculate_time_bonus(
            start_time,
            end_time,
            user_bet.creation_ts
        )?;

        let conviction_bonus = calculate_conviction_bonus(user_bet.update_count);

        let mut weight = calculate_weight(
            user_bet.deposit,
            accuracy_score,
            time_bonus,
            conviction_bonus
        )?;

        let penalty = weight.checked_div(20).unwrap(); 
        weight = weight.checked_sub(penalty).unwrap();

        pool.total_weight = pool.total_weight.checked_add(weight).unwrap();
        
        user_bet.calculated_weight = weight;
        user_bet.is_weight_added = true; 
        user_bet.status = BetStatus::Calculated;

        let mut new_data: Vec<u8> = Vec::new();
        user_bet.try_serialize(&mut new_data)?;

        if new_data.len() <= user_bet_data.len() {
            user_bet_data[..new_data.len()].copy_from_slice(&new_data);
        } else {
            return Err(ProgramError::AccountDataTooSmall.into());
        }

        emit!(OutcomeCalculated {
            bet_address: user_bet_acc_info.key(),
            user: user_bet.owner,
            weight: weight,
        });
    }

    Ok(())
}