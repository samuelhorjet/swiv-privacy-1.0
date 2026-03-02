use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{Bet, Pool, BetStatus};
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
        constraint = bet.user_pubkey == user.key() @ CustomError::Unauthorized,
        constraint = bet.status != BetStatus::Claimed @ CustomError::AlreadyClaimed
    )]
    pub bet: Box<Account<'info, Bet>>,

    #[account(
        mut,
        seeds = [SEED_POOL, pool.created_by.as_ref(), &(pool.pool_id.to_le_bytes())],
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
        token::mint = pool.stake_token_mint
    )]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

pub fn emergency_refund(ctx: Context<EmergencyRefund>) -> Result<()> {
    let bet = &mut ctx.accounts.bet;
    let pool = &mut ctx.accounts.pool;
    let clock = Clock::get()?;

    require!(
        clock.unix_timestamp > bet.end_timestamp + REFUND_TIMEOUT_SECONDS,
        CustomError::TimeoutNotMet
    );


    let refund_amount = bet.stake;

    if refund_amount > 0 {
        let created_by_bytes = pool.created_by.as_ref();
        let pool_id_bytes = pool.pool_id.to_le_bytes();
        let bump = pool.bump;
        let seeds = &[SEED_POOL, created_by_bytes, &pool_id_bytes, &[bump]];
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
        
        pool.total_volume = pool.total_volume.checked_sub(refund_amount).unwrap();
    }

    bet.status = BetStatus::Claimed;
    
    emit!(BetRefunded {
        bet_address: bet.key(),
        user: ctx.accounts.user.key(),
        amount: refund_amount,
        is_emergency: true,
    });

    msg!("Emergency Refund executed for user: {}", ctx.accounts.user.key());

    Ok(())
}