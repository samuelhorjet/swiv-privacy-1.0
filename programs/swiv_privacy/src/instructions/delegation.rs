use anchor_lang::prelude::*;
use crate::state::{UserBet, GlobalConfig, Pool}; 
use crate::constants::{SEED_BET, SEED_POOL, SEED_GLOBAL_CONFIG}; 
use crate::errors::CustomError;
use crate::events::{
    PoolDelegated, PoolUndelegated, 
    BetDelegated, BetUndelegated
}; 
use ephemeral_rollups_sdk::anchor::{delegate, commit};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;

#[delegate]
#[derive(Accounts)]
#[instruction(pool_name: String)]
pub struct DelegatePool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [SEED_GLOBAL_CONFIG],
        bump,
        constraint = global_config.admin == admin.key() @ CustomError::Unauthorized
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// CHECK: The main pool account.
   #[account(
        mut, 
        del, 
        seeds = [SEED_POOL, pool_name.as_bytes()],
        bump
    )]
    pub pool: AccountInfo<'info>,
}

pub fn delegate_pool(ctx: Context<DelegatePool>, pool_name: String) -> Result<()> {
    let seeds = &[
        SEED_POOL,
        pool_name.as_bytes(),
    ];

    ctx.accounts.delegate_pool(
        &ctx.accounts.admin, 
        seeds, 
        DelegateConfig::default(),             
    )?;

    emit!(PoolDelegated {
        pool_address: ctx.accounts.pool.key(),
    });

    msg!("Pool account delegated successfully.");
    Ok(())
}


// --- BET DELEGATION ---

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
}

pub fn delegate_bet(ctx: Context<DelegateBet>, request_id: String) -> Result<()> {
    // Deserialize just enough to verify ownership
    let (pool_identifier, owner) = {
        let user_bet_data = ctx.accounts.user_bet.try_borrow_data()?;
        let mut data_slice: &[u8] = &user_bet_data;
        let user_bet = UserBet::try_deserialize(&mut data_slice)?;
        (user_bet.pool_identifier, user_bet.owner)
    }; 

    require!(owner == ctx.accounts.user.key(), CustomError::Unauthorized);
    
    // Verify the pool address matches the bet's pool_identifier
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
    
    ctx.accounts.delegate_user_bet(
        &ctx.accounts.user, 
        seeds_for_sdk, 
        DelegateConfig::default(),             
    )?;

    emit!(BetDelegated {
        bet_address: ctx.accounts.user_bet.key(),
        user: ctx.accounts.user.key(),
        request_id,
    });

    msg!("Bet delegated successfully.");
    Ok(())
}


// --- UNDELEGATION ---

#[commit]
#[derive(Accounts)]
pub struct UndelegatePool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [SEED_GLOBAL_CONFIG],
        bump,
        constraint = global_config.admin == admin.key() @ CustomError::Unauthorized
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
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
        seeds = [SEED_POOL, pool.name.as_bytes()],
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