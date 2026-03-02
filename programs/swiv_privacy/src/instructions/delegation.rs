use anchor_lang::prelude::*;
use crate::state::{Bet, Protocol, Pool}; 
use crate::constants::{SEED_BET, SEED_POOL, SEED_PROTOCOL}; 
use crate::errors::CustomError;
use crate::events::{
    PoolDelegated, PoolUndelegated, 
    BetDelegated, BetUndelegated
}; 
use ephemeral_rollups_sdk::access_control::instructions::DelegatePermissionCpiBuilder;

use ephemeral_rollups_sdk::anchor::{delegate, commit};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;

#[delegate]
#[derive(Accounts)]
#[instruction(pool_id: u64)]
pub struct DelegatePool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [SEED_PROTOCOL],
        bump,
        constraint = protocol.admin == admin.key() @ CustomError::Unauthorized
    )]
    pub protocol: Account<'info, Protocol>,

    /// CHECK: The main pool account.
   #[account(
        mut, 
        del, 
        seeds = [SEED_POOL, admin.key().as_ref(), &pool_id.to_le_bytes()],
        bump
    )]
    pub pool: AccountInfo<'info>,

    /// CHECK: The MagicBlock Ephemeral Rollup Validator (TEE)
    pub validator: UncheckedAccount<'info>,
}

pub fn delegate_pool(ctx: Context<DelegatePool>, pool_id: u64) -> Result<()> {
    let admin_key = ctx.accounts.admin.key();
    let admin_bytes = admin_key.as_ref();
    let pool_id_bytes = pool_id.to_le_bytes();
    let seeds = &[
        SEED_POOL,
        admin_bytes,
        &pool_id_bytes,
    ];

    let config = DelegateConfig {
        validator: Some(ctx.accounts.validator.key()),
        ..DelegateConfig::default()
    };

    ctx.accounts.delegate_pool(
        &ctx.accounts.admin, 
        seeds, 
        config,             
    )?;

    emit!(PoolDelegated {
        pool_address: ctx.accounts.pool.key(),
    });

    msg!("Pool account delegated successfully.");
    Ok(())
}

#[derive(Accounts)]
#[instruction(request_id: String)]
pub struct DelegateBetPermission<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Manually validated against the bet's pool_identifier.
    pub pool: AccountInfo<'info>,

    /// CHECK: The user's bet account (The Permissioned Account)
    #[account(mut)]
    pub user_bet: AccountInfo<'info>,

    /// CHECK: The permission account associated with the user_bet.
    #[account(mut)]
    pub permission: UncheckedAccount<'info>,

    /// CHECK: The MagicBlock Permission Program
    pub permission_program: UncheckedAccount<'info>,

    /// CHECK: The MagicBlock Delegation Program
    pub delegation_program: UncheckedAccount<'info>,

    /// CHECK: Delegation buffer (Derived by client or SDK)
    #[account(mut)]
    pub delegation_buffer: UncheckedAccount<'info>,

    /// CHECK: Delegation record (Derived by client or SDK)
    #[account(mut)]
    pub delegation_record: UncheckedAccount<'info>,

    /// CHECK: Delegation metadata (Derived by client or SDK)
    #[account(mut)]
    pub delegation_metadata: UncheckedAccount<'info>,

    /// CHECK: The MagicBlock Ephemeral Rollup Validator (TEE)
    pub validator: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn delegate_bet_permission(ctx: Context<DelegateBetPermission>, request_id: String) -> Result<()> {
    let (pool_pubkey, owner, bump) = {
        let user_bet_data = ctx.accounts.user_bet.try_borrow_data()?;
        let mut data_slice: &[u8] = &user_bet_data;
        let bet = Bet::try_deserialize(&mut data_slice)?;
        (bet.pool_pubkey, bet.user_pubkey, bet.bump)
    };

    require!(owner == ctx.accounts.user.key(), CustomError::Unauthorized);
    require!(pool_pubkey == ctx.accounts.pool.key(), CustomError::PoolMismatch);

    let pool_key = ctx.accounts.pool.key();
    let user_key = ctx.accounts.user.key();

    let seeds_for_signing = &[
        SEED_BET,
        pool_key.as_ref(), 
        user_key.as_ref(),
        request_id.as_bytes(),
        &[bump],
    ];
    let signer_seeds = &[&seeds_for_signing[..]];

    DelegatePermissionCpiBuilder::new(&ctx.accounts.permission_program)
        .payer(&ctx.accounts.user)
        .authority(&ctx.accounts.user, false) 
        .permissioned_account(&ctx.accounts.user_bet, true) // user_bet signs
        .permission(&ctx.accounts.permission)
        .system_program(&ctx.accounts.system_program)
        .owner_program(&ctx.accounts.permission_program)
        .delegation_buffer(&ctx.accounts.delegation_buffer)
        .delegation_record(&ctx.accounts.delegation_record)
        .delegation_metadata(&ctx.accounts.delegation_metadata)
        .delegation_program(&ctx.accounts.delegation_program)
        .validator(Some(&ctx.accounts.validator))
        .invoke_signed(signer_seeds)?;
    msg!("Permission account delegated successfully.");
    Ok(())
}

#[delegate]
#[derive(Accounts)]
#[instruction(request_id: String)]
pub struct DelegateBet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Manually validated against the bet's pool_identifier.
    pub pool: AccountInfo<'info>,

    /// CHECK: The user's bet account.
    #[account(mut, del)]
    pub user_bet: AccountInfo<'info>,

    /// CHECK: The MagicBlock Ephemeral Rollup Validator (TEE)
    pub validator: UncheckedAccount<'info>,
}

pub fn delegate_bet(ctx: Context<DelegateBet>, request_id: String) -> Result<()> {
    let (pool_pubkey, owner) = {
        let user_bet_data = ctx.accounts.user_bet.try_borrow_data()?;
        let mut data_slice: &[u8] = &user_bet_data;
        let bet = Bet::try_deserialize(&mut data_slice)?;
        (bet.pool_pubkey, bet.user_pubkey)
    }; 

    require!(owner == ctx.accounts.user.key(), CustomError::Unauthorized);
    require!(pool_pubkey == ctx.accounts.pool.key(), CustomError::PoolMismatch);

    let pool_key = ctx.accounts.pool.key();
    let user_key = ctx.accounts.user.key();

    let seeds_for_sdk = &[
        SEED_BET,
        pool_key.as_ref(), 
        user_key.as_ref(),
        request_id.as_bytes(),
    ];
    
    let config = DelegateConfig {
        validator: Some(ctx.accounts.validator.key()),
        ..DelegateConfig::default()
    };

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

    msg!("Bet delegated successfully.");
    Ok(())
}

#[commit]
#[derive(Accounts)]
pub struct UndelegatePool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [SEED_PROTOCOL],
        bump,
        constraint = protocol.admin == admin.key() @ CustomError::Unauthorized
    )]
    pub protocol: Account<'info, Protocol>,
    
    /// CHECK: The Pool account
    #[account(mut)]
    pub pool: AccountInfo<'info>,
}

pub fn undelegate_pool(ctx: Context<UndelegatePool>) -> Result<()> {
    commit_and_undelegate_accounts(
        &ctx.accounts.admin,
        vec![&ctx.accounts.pool],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;

    emit!(PoolUndelegated {
        pool_address: ctx.accounts.pool.key(),
    });

    Ok(())
}

#[commit]
#[derive(Accounts)]
pub struct BatchUndelegateBets<'info> {
    #[account(mut)]
    pub payer: Signer<'info>, 

    #[account(
        mut,
        seeds = [SEED_POOL, pool.created_by.as_ref(), &(pool.pool_id.to_le_bytes())],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,
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

    msg!("Batch Undelegate executed for {} bets.", ctx.remaining_accounts.len());
    Ok(())
}