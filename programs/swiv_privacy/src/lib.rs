use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("5e4e9pooJgus8NcgdwZitZx1kF2vWZEA7MCTGi7BuKag");

#[ephemeral]
#[program]
pub mod swiv_privacy {
    use super::*;

    // --- ADMIN ---
    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        protocol_fee_bps: u64,
    ) -> Result<()> {
        admin::initialize_protocol(ctx, protocol_fee_bps)
    }

    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_treasury: Option<Pubkey>,
        new_protocol_fee_bps: Option<u64>,
    ) -> Result<()> {
        admin::update_config(ctx, new_treasury, new_protocol_fee_bps)
    }

    pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
        admin::transfer_admin(ctx, new_admin)
    }

    pub fn set_pause(ctx: Context<SetPause>, paused: bool) -> Result<()> {
        admin::set_pause(ctx, paused)
    }

    // --- DELEGATION ---
    pub fn delegate_pool(ctx: Context<DelegatePool>, pool_id: u64) -> Result<()> {
        instructions::delegation::delegate_pool(ctx, pool_id)
    }

    pub fn undelegate_pool(ctx: Context<UndelegatePool>) -> Result<()> {
        instructions::delegation::undelegate_pool(ctx)
    }

    pub fn delegate_bet(ctx: Context<DelegateBet>, request_id: String) -> Result<()> {
        instructions::delegation::delegate_bet(ctx, request_id)
    }

    pub fn batch_undelegate_bets<'info>(
        ctx: Context<'_, '_, '_, 'info, BatchUndelegateBets<'info>>,
    ) -> Result<()> {
        instructions::delegation::batch_undelegate_bets(ctx)
    }

    pub fn create_bet_permission(ctx: Context<CreateBetPermission>, req_id: String) -> Result<()> {
        instructions::permission::create_bet_permission(ctx, req_id)
    }

    pub fn delegate_bet_permission<'info>(
        ctx: Context<DelegateBetPermission>,
        request_id: String,
    ) -> Result<()> {
        instructions::delegation::delegate_bet_permission(ctx, request_id)
    }

    // --- POOL ---
    pub fn create_pool(
        ctx: Context<CreatePool>,
        pool_id: u64,
        name: String,
        metadata: Option<String>,
        start_time: i64,
        end_time: i64,
        max_accuracy_buffer: u64,
        conviction_bonus_bps: u64,
    ) -> Result<()> {
        pool::create_pool(
            ctx,
            pool_id,
            name,
            metadata,
            start_time,
            end_time,
            max_accuracy_buffer,
            conviction_bonus_bps,
        )
    }

    // --- BET ---
    pub fn place_bet(ctx: Context<PlaceBet>, prediction: u64, request_id: String) -> Result<()> {
        pool::place_bet(ctx, prediction, request_id)
    }

    pub fn init_bet(ctx: Context<InitBet>, amount: u64, request_id: String) -> Result<()> {
        pool::init_bet(ctx, amount, request_id)
    }
    pub fn resolve_pool(ctx: Context<ResolvePool>, final_outcome: u64) -> Result<()> {
        pool::resolve_pool(ctx, final_outcome)
    }

    pub fn batch_calculate_weights<'info>(
        ctx: Context<'_, '_, '_, 'info, BatchCalculateWeights<'info>>,
    ) -> Result<()> {
        admin::batch_calculate_weights(ctx)
    }

    pub fn finalize_weights(ctx: Context<FinalizeWeights>) -> Result<()> {
        pool::finalize_weights(ctx)
    }

    pub fn claim_reward(ctx: Context<ClaimReward>) -> Result<()> {
        pool::claim_reward(ctx)
    }

    pub fn update_bet(ctx: Context<UpdateBet>, new_prediction: u64) -> Result<()> {
        pool::update_bet(ctx, new_prediction)
    }

    pub fn emergency_refund(ctx: Context<EmergencyRefund>) -> Result<()> {
        pool::emergency_refund(ctx)
    }
}
