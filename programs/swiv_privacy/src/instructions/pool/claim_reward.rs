use crate::constants::{SEED_POOL, SEED_POOL_VAULT};
use crate::errors::CustomError;
use crate::state::{BetStatus, Pool, Bet};
use crate::events::RewardClaimed;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct ClaimReward<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

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
    pub pool_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = bet.user_pubkey == user.key() @ CustomError::Unauthorized,
    )]
    pub bet: Box<Account<'info, Bet>>,

    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn claim_reward(ctx: Context<ClaimReward>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let bet = &mut ctx.accounts.bet;
    let mut payout_amount: u64 = 0;

    require!(pool.weight_finalized, CustomError::SettlementTooEarly);

    if bet.calculated_weight > 0 && pool.total_weight > 0 {
        let total_distributable_pot = pool.total_volume as u128;

        payout_amount = bet
            .calculated_weight
            .checked_mul(total_distributable_pot)
            .unwrap()
            .checked_div(pool.total_weight)
            .unwrap() as u64;
    }

    if payout_amount > 0 {
        require!(
            payout_amount <= pool.total_volume,
            CustomError::InsufficientLiquidity
        );

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
            payout_amount,
        )?;
        
    }

    bet.status = BetStatus::Claimed;

    emit!(RewardClaimed {
        bet_address: bet.key(),
        user: ctx.accounts.user.key(),
        amount: payout_amount,
    });

    Ok(())
}