use crate::constants::SEED_BET;
use crate::errors::CustomError;
use anchor_lang::prelude::*;

use ephemeral_rollups_sdk::access_control::instructions::CreatePermissionCpiBuilder;
use ephemeral_rollups_sdk::access_control::structs::{Member, MembersArgs, AUTHORITY_FLAG};

#[derive(Accounts)]
#[instruction(request_id: String)]
pub struct CreateBetPermission<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: The user who is given authority
    pub user: UncheckedAccount<'info>,

    /// CHECK: We manually verify seeds below to invoke with canonical bump
    pub user_bet: UncheckedAccount<'info>,

    /// CHECK: Passed to permission program. Must be UncheckedAccount.
    pub pool: UncheckedAccount<'info>,

    /// CHECK: Validated by Permission Program
    #[account(mut)]
    pub permission: UncheckedAccount<'info>,

    /// CHECK: The MagicBlock Permission Program ID
    pub permission_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_bet_permission(ctx: Context<CreateBetPermission>, request_id: String) -> Result<()> {
    let pool_key = ctx.accounts.pool.key();
    let user_key = ctx.accounts.user.key();

    let seeds_no_bump: Vec<Vec<u8>> = vec![
        SEED_BET.to_vec(),
        pool_key.to_bytes().to_vec(),
        user_key.to_bytes().to_vec(),
        request_id.as_bytes().to_vec(),
    ];

    let (derived_pda, bump) = Pubkey::find_program_address(
        &seeds_no_bump
            .iter()
            .map(|s| s.as_slice())
            .collect::<Vec<_>>(),
        &crate::ID,
    );

    require!(
        derived_pda == ctx.accounts.user_bet.key(),
        CustomError::SeedMismatch
    );

    let mut seeds = seeds_no_bump.clone();
    seeds.push(vec![bump]);
    let seed_refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
    let signer_seeds = &[seed_refs.as_slice()];

    let member = Member {
        pubkey: ctx.accounts.user.key(),
        flags: AUTHORITY_FLAG,
    };
    let args = MembersArgs {
        members: Some(vec![member]),
    };

    CreatePermissionCpiBuilder::new(&ctx.accounts.permission_program)
        .payer(&ctx.accounts.payer)
        .system_program(&ctx.accounts.system_program)
        .permission(&ctx.accounts.permission)
        .permissioned_account(&ctx.accounts.user_bet)
        .args(args)
        .invoke_signed(signer_seeds)?;

    Ok(())
}
