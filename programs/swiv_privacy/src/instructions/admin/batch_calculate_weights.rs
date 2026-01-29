use crate::constants::SEED_POOL;
use crate::errors::CustomError;
use crate::events::OutcomeCalculated;
use crate::state::{BetStatus, Pool, UserBet};
use crate::utils::math::{
    calculate_accuracy_score, calculate_conviction_bonus, calculate_time_bonus, calculate_weight,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct BatchCalculateWeights<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [
            SEED_POOL, 
            pool.admin.as_ref(), 
            &pool.pool_id.to_le_bytes()
        ],
        bump = pool.bump,
    )]
    pub pool: Account<'info, Pool>,
}

pub fn batch_calculate_weights<'info>(
    ctx: Context<'_, '_, '_, 'info, BatchCalculateWeights<'info>>,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let accounts_iter = &mut ctx.remaining_accounts.iter();

    require!(pool.is_resolved, CustomError::SettlementTooEarly);
    require!(!pool.weight_finalized, CustomError::AlreadyClaimed);

    let result = pool.resolution_target;
    let start_time = pool.start_time;
    let end_time = pool.end_time;
    let max_accuracy_buffer = pool.max_accuracy_buffer;

    for user_bet_acc_info in accounts_iter {
        let mut user_bet_data = user_bet_acc_info.try_borrow_mut_data()?;
        let mut user_bet = UserBet::try_deserialize(&mut &user_bet_data[..])?;

        if user_bet.pool != pool.key() { continue; }
        
        if user_bet.status == BetStatus::Calculated || user_bet.status == BetStatus::Claimed { 
            continue; 
        }

        let accuracy_score = calculate_accuracy_score(
            user_bet.prediction, 
            result, 
            max_accuracy_buffer
        )?;
        
        let time_bonus = calculate_time_bonus(
            start_time, 
            end_time, 
            user_bet.creation_ts
        )?;
        
        let conviction_bonus = calculate_conviction_bonus(user_bet.update_count);
        
        let weight = calculate_weight(
            user_bet.deposit,
            accuracy_score,
            time_bonus,
            conviction_bonus,
        )?;

        pool.total_weight = pool.total_weight.checked_add(weight).unwrap();
        
        user_bet.calculated_weight = weight;
        user_bet.is_weight_added = true;
        user_bet.status = BetStatus::Calculated;

        let mut new_data: Vec<u8> = Vec::new();
        user_bet.try_serialize(&mut new_data)?;
        user_bet_data[..new_data.len()].copy_from_slice(&new_data);

        emit!(OutcomeCalculated {
            bet_address: user_bet_acc_info.key(),
            user: user_bet.owner,
            weight: weight,
        });
    }

    Ok(())
}