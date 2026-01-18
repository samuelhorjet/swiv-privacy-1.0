use crate::errors::CustomError;
use anchor_lang::prelude::*;

pub const MATH_PRECISION: u128 = 1_000_000; 

pub fn calculate_accuracy_score(
    prediction: u64,
    result: u64,
    buffer: u64,
) -> Result<u64> {
    if buffer == 0 {
        return Ok(0);
    }

    let diff = if prediction > result {
        prediction - result
    } else {
        result - prediction
    };

    if diff >= buffer {
        return Ok(0);
    }
    
    let diff_u128 = diff as u128;
    let buffer_u128 = buffer as u128;

    let error_fraction = diff_u128
        .checked_mul(MATH_PRECISION)
        .ok_or(CustomError::MathOverflow)?
        .checked_div(buffer_u128)
        .ok_or(CustomError::MathOverflow)?;

    let score = MATH_PRECISION.saturating_sub(error_fraction);

    Ok(score as u64)
}

pub fn calculate_time_bonus(
    start_time: i64,
    end_time: i64,
    entry_time: i64,
) -> Result<u64> {
    if entry_time >= end_time {
        return Ok(MATH_PRECISION as u64);
    }

    let total_duration = (end_time - start_time) as u128;
    let remaining_time = (end_time - entry_time) as u128;

    if total_duration == 0 {
        return Ok(MATH_PRECISION as u64);
    }

    let bonus_portion = remaining_time
        .checked_mul(MATH_PRECISION)
        .unwrap()
        .checked_div(total_duration)
        .unwrap();

    let factor = MATH_PRECISION.checked_add(bonus_portion).unwrap();

    Ok(factor as u64)
}

pub fn calculate_conviction_bonus(update_count: u32) -> u64 {
    if update_count == 0 {
        1_500_000
    } else {
        1_000_000
    }
}

pub fn calculate_weight(
    stake: u64,
    accuracy_score_scaled: u64,
    time_bonus_scaled: u64,
    conviction_scaled: u64,
) -> Result<u128> {
    
    let stake_u128 = stake as u128;
    
    let raw_product = stake_u128
        .checked_mul(accuracy_score_scaled as u128).unwrap()
        .checked_mul(time_bonus_scaled as u128).unwrap()
        .checked_mul(conviction_scaled as u128).unwrap();

    let final_weight = raw_product
        .checked_div(MATH_PRECISION).unwrap()
        .checked_div(MATH_PRECISION).unwrap()
        .checked_div(MATH_PRECISION).unwrap();

    Ok(final_weight)
}