use anchor_lang::prelude::*;
use solana_program::keccak; 
use crate::state::{UserBet}; 
use crate::errors::CustomError;
use crate::events::BetRevealed;

#[derive(Accounts)]
pub struct RevealBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_bet.owner == user.key() @ CustomError::Unauthorized,
        constraint = !user_bet.is_revealed @ CustomError::AlreadyRevealed
    )]
    pub user_bet: Account<'info, UserBet>,
}

pub fn reveal_bet(
    ctx: Context<RevealBet>,
    prediction_target: u64,
    salt: [u8; 32], 
) -> Result<()> {
    let user_bet = &mut ctx.accounts.user_bet;
    let clock = Clock::get()?;

    let max_delay_seconds = 300; 
    if clock.unix_timestamp > user_bet.creation_ts + max_delay_seconds {
        return Err(CustomError::RevealWindowExpired.into());
    }

    let mut data = Vec::new();
    data.extend_from_slice(&prediction_target.to_le_bytes());
    data.extend_from_slice(&salt);

    let calculated_hash = keccak::hash(&data);

    require!(
        calculated_hash.to_bytes() == user_bet.commitment,
        CustomError::InvalidCommitment
    );

    user_bet.prediction_target = prediction_target;
    user_bet.is_revealed = true;

    emit!(BetRevealed {
        bet_address: user_bet.key(),
        decrypted_target: prediction_target
    });

    msg!("Bet Revealed Successfully");
    Ok(())
}