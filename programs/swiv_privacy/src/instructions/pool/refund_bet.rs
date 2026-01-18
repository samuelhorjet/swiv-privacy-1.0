use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{UserBet, Pool, GlobalConfig, BetStatus};
use crate::constants::{SEED_POOL, SEED_POOL_VAULT, SEED_GLOBAL_CONFIG};
use crate::errors::CustomError;
use crate::events::BetRefunded;

#[derive(Accounts)]
pub struct RefundBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_bet.owner == user.key() @ CustomError::Unauthorized,
        constraint = user_bet.status != BetStatus::Settled @ CustomError::AlreadySettled
    )]
    pub user_bet: Box<Account<'info, UserBet>>,
    
    #[account(
        seeds = [SEED_GLOBAL_CONFIG],
        bump,
    )]
    pub global_config: Box<Account<'info, GlobalConfig>>,

    /// CHECK: Must match global config treasury wallet
    #[account(address = global_config.treasury_wallet)]
    pub treasury_wallet: UncheckedAccount<'info>,

    #[account(
        mut,
        token::authority = treasury_wallet
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [SEED_POOL, user_bet.pool_identifier.as_bytes()],
        bump = pool.bump 
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        seeds = [SEED_POOL_VAULT, pool.key().as_ref()],
        bump,
        token::authority = pool,
    )]
    pub pool_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn refund_bet(ctx: Context<RefundBet>) -> Result<()> {
    let clock = Clock::get()?;
    let user_bet = &mut ctx.accounts.user_bet;
    
    require!(!user_bet.is_revealed, CustomError::CannotRefundRevealed);
    require!(clock.unix_timestamp > user_bet.end_timestamp, CustomError::SettlementTooEarly);

    let penalty_bps = 100u64;
    let penalty_amount = user_bet.deposit
        .checked_mul(penalty_bps).unwrap()
        .checked_div(10_000).unwrap();

    let refund_amount = user_bet.deposit.checked_sub(penalty_amount).unwrap();

    let pool = &mut ctx.accounts.pool;
    let pool_vault = &ctx.accounts.pool_vault;

    let bump = pool.bump;
    
    let seeds = &[SEED_POOL, pool.name.as_bytes(), &[bump]]; 
    let signer = &[&seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: pool_vault.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: pool.to_account_info(),
            },
            signer,
        ),
        refund_amount,
    )?;

    if penalty_amount > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: pool_vault.to_account_info(),
                    to: ctx.accounts.treasury_token_account.to_account_info(), 
                    authority: pool.to_account_info(),
                },
                signer,
            ),
            penalty_amount,
        )?;
    }
    pool.vault_balance = pool.vault_balance.checked_sub(user_bet.deposit).unwrap();

    user_bet.status = BetStatus::Settled;
    
    emit!(BetRefunded {
        bet_address: user_bet.key(),
        user: ctx.accounts.user.key(),
        amount: refund_amount,
        is_emergency: false,
    });

    msg!("Refund Complete. Refund: {}, Penalty: {}", refund_amount, penalty_amount);

    Ok(())
}