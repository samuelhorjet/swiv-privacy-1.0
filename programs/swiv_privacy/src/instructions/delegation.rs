use anchor_lang::prelude::*;
use crate::state::{UserBet, Pool};
use crate::constants::{SEED_BET, SEED_POOL};
use crate::errors::CustomError;
use crate::events::{BetDelegated, BetUndelegated};
use ephemeral_rollups_sdk::anchor::{delegate, commit};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;

#[delegate]
#[derive(Accounts)]
#[instruction(request_id: String)]
pub struct DelegateBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Validated manually against user_bet.pool_identifier
    pub pool: AccountInfo<'info>,

    /// CHECK: We use AccountInfo to work with the #[delegate] macro
    #[account(mut, del)]
    pub user_bet: AccountInfo<'info>,
}

pub fn delegate_bet(ctx: Context<DelegateBet>, request_id: String) -> Result<()> {
    let (_bump, pool_identifier, owner) = {
        let user_bet_data = ctx.accounts.user_bet.try_borrow_data()?;
        let mut data_slice: &[u8] = &user_bet_data;
        let user_bet = UserBet::try_deserialize(&mut data_slice)?;
        (user_bet.bump, user_bet.pool_identifier, user_bet.owner)
    }; 

    require!(owner == ctx.accounts.user.key(), CustomError::Unauthorized);
    
    let (pool_pda, _) = Pubkey::find_program_address(
        &[SEED_POOL, pool_identifier.as_bytes()], 
        &crate::ID
    );
    require!(pool_pda == ctx.accounts.pool.key(), CustomError::PoolMismatch);

    let pool_key = ctx.accounts.pool.key();
    let user_key = ctx.accounts.user.key();
    
    let seeds_for_sdk = &[
        SEED_BET,
        pool_key.as_ref(),
        user_key.as_ref(),
        request_id.as_bytes(),
    ];

    let config = DelegateConfig::default();
    
    ctx.accounts.delegate_user_bet(
        &ctx.accounts.user, 
        seeds_for_sdk, 
        config,             
    )?;

    emit!(BetDelegated {
        bet_address: ctx.accounts.user_bet.key(),
        user: ctx.accounts.user.key(),
        request_id,
    });

    msg!("Bet Delegated successfully");
    Ok(())
}

#[commit]
#[derive(Accounts)]
#[instruction(request_id: String)]
pub struct UndelegateBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [SEED_POOL, user_bet.pool_identifier.as_bytes()],
        bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        seeds = [
            SEED_BET, 
            pool.key().as_ref(), 
            user.key().as_ref(), 
            request_id.as_bytes()
        ],
        bump = user_bet.bump,
        constraint = user_bet.owner == user.key() @ CustomError::Unauthorized,
    )]
    pub user_bet: Box<Account<'info, UserBet>>,
}

pub fn undelegate_bet(ctx: Context<UndelegateBet>, _request_id: String) -> Result<()> {
    let pool = &ctx.accounts.pool;
    let clock = Clock::get()?;

    require!(
        clock.unix_timestamp >= pool.end_time,
        CustomError::UndelegationTooEarly
    );
    
    commit_and_undelegate_accounts(
        &ctx.accounts.user,
        vec![&ctx.accounts.user_bet.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;

    emit!(BetUndelegated {
        bet_address: ctx.accounts.user_bet.key(),
        user: ctx.accounts.user.key(),
        is_batch: false,
    });

    msg!("Bet Undelegated (Committed)");
    Ok(())
}

#[commit]
#[derive(Accounts)]
pub struct BatchUndelegateBets<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [SEED_POOL, pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,
}

pub fn batch_undelegate_bets<'info>(ctx: Context<'_, '_, '_, 'info, BatchUndelegateBets<'info>>) -> Result<()> {
    let pool = &ctx.accounts.pool;
    let clock = Clock::get()?;

    require!(
        clock.unix_timestamp >= pool.end_time,
        CustomError::UndelegationTooEarly
    );
    
    let accounts_to_undelegate: Vec<&AccountInfo<'info>> = ctx.remaining_accounts.iter().collect();
    
    if accounts_to_undelegate.is_empty() {
        return Ok(());
    }

    commit_and_undelegate_accounts(
        &ctx.accounts.payer,
        accounts_to_undelegate,
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;

    for acc in ctx.remaining_accounts.iter() {
        emit!(BetUndelegated {
            bet_address: acc.key(),
            user: Pubkey::default(),
            is_batch: true,
        });
    }

    msg!("Batch Undelegate Executed for {} bets", ctx.remaining_accounts.len());
    Ok(())
}