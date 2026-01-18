use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{GlobalConfig, Pool, UserBet, BetStatus};
use crate::constants::{SEED_BET, SEED_POOL, SEED_POOL_VAULT, SEED_GLOBAL_CONFIG}; 
use crate::errors::CustomError;
use crate::events::BetPlaced;

#[derive(Accounts)]
#[instruction(
    amount: u64,
    commitment: [u8; 32], 
    request_id: String
)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [SEED_GLOBAL_CONFIG],
        bump,
        constraint = !global_config.paused @ CustomError::Paused
    )]
    pub global_config: Box<Account<'info, GlobalConfig>>,

    #[account(
        mut,
        seeds = [SEED_POOL, pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        seeds = [SEED_POOL_VAULT, pool.key().as_ref()],
        bump,
        token::authority = pool,
    )]
    pub pool_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = user,
        space = UserBet::SPACE,
        seeds = [SEED_BET, pool.key().as_ref(), user.key().as_ref(), request_id.as_bytes()], 
        bump
    )]
    pub user_bet: Box<Account<'info, UserBet>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn place_bet(
    ctx: Context<PlaceBet>,
    amount: u64,
    commitment: [u8; 32], 
    _request_id: String, 
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    require!(clock.unix_timestamp >= pool.start_time, CustomError::DurationTooShort);
    require!(clock.unix_timestamp < pool.end_time, CustomError::DurationTooShort); 

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.pool_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
    )?;

    pool.vault_balance = pool.vault_balance.checked_add(amount).unwrap();

    {
        let user_bet = &mut ctx.accounts.user_bet;
        user_bet.owner = ctx.accounts.user.key();
        user_bet.pool_identifier = pool.name.clone();
        user_bet.deposit = amount; 
        user_bet.end_timestamp = pool.end_time;
        
        user_bet.creation_ts = clock.unix_timestamp; 
        user_bet.update_count = 0;                   
        user_bet.calculated_weight = 0;
        user_bet.is_weight_added = false;
        user_bet.status = BetStatus::Active;
        
        user_bet.commitment = commitment;
        user_bet.is_revealed = false;
        user_bet.prediction_target = 0;
        
        user_bet.bump = ctx.bumps.user_bet;
    }

    emit!(BetPlaced {
        bet_address: ctx.accounts.user_bet.key(),
        user: ctx.accounts.user.key(),
        pool_identifier: pool.name.clone(),
        amount,
        end_timestamp: pool.end_time,
    });

    Ok(())
}