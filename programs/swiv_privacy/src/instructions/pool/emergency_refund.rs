use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{UserBet, Pool, BetStatus};
use crate::constants::{SEED_POOL, SEED_POOL_VAULT};
use crate::errors::CustomError;
use crate::events::BetRefunded;

const REFUND_TIMEOUT_SECONDS: i64 = 60; 

#[derive(Accounts)]
pub struct EmergencyRefund<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_bet.owner == user.key() @ CustomError::Unauthorized,
        constraint = user_bet.status != BetStatus::Settled @ CustomError::AlreadySettled
    )]
    pub user_bet: Box<Account<'info, UserBet>>,

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

    #[account(
        mut,
        token::mint = pool.token_mint
    )]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

pub fn emergency_refund(ctx: Context<EmergencyRefund>) -> Result<()> {
    let user_bet = &mut ctx.accounts.user_bet;
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    require!(
        clock.unix_timestamp > user_bet.end_timestamp + REFUND_TIMEOUT_SECONDS,
        CustomError::TimeoutNotMet
    );


    let refund_amount = user_bet.deposit;

    if refund_amount > 0 {
        let name_bytes = pool.name.as_bytes();
        let bump = pool.bump;
        let seeds = &[SEED_POOL, name_bytes, &[bump]];
        let signer = &[&seeds[..]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_vault.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: pool.to_account_info(),
                },
                signer,
            ),
            refund_amount,
        )?;
        
        pool.vault_balance = pool.vault_balance.checked_sub(refund_amount).unwrap();
    }

    user_bet.status = BetStatus::Settled;
    
    emit!(BetRefunded {
        bet_address: user_bet.key(),
        user: ctx.accounts.user.key(),
        amount: refund_amount,
        is_emergency: true,
    });

    msg!("Emergency Refund executed for user: {}", ctx.accounts.user.key());

    Ok(())
}