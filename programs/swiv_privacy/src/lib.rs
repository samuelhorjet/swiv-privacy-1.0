use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("8aAVXhM9uNdoijr7YJf3KG8yikHYdAUQZ6kmts9BBVLh");

#[ephemeral]
#[program]
pub mod swiv_privacy {
    use super::*;

    // --- ADMIN & CONFIG ---
    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>, 
        protocol_fee_bps: u64 
    ) -> Result<()> {
        admin::initialize_protocol(ctx, protocol_fee_bps)
    }

    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_treasury: Option<Pubkey>,
        new_protocol_fee_bps: Option<u64>, 
    ) -> Result<()> {
        admin::update_config(
            ctx, 
            new_treasury, 
            new_protocol_fee_bps, 
        )
    }

    pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
        admin::transfer_admin(ctx, new_admin)
    }

    pub fn set_pause(ctx: Context<SetPause>, paused: bool) -> Result<()> {
        admin::set_pause(ctx, paused)
    }

    // --- DELEGATION ---
    pub fn delegate_bet(ctx: Context<DelegateBet>, request_id: String) -> Result<()> {
        instructions::delegation::delegate_bet(ctx, request_id)
    }

    pub fn undelegate_bet(ctx: Context<UndelegateBet>, request_id: String) -> Result<()> {
        instructions::delegation::undelegate_bet(ctx, request_id)
    }

    pub fn batch_undelegate_bets<'info>(
        ctx: Context<'_, '_, '_, 'info, BatchUndelegateBets<'info>>
    ) -> Result<()> {
        instructions::delegation::batch_undelegate_bets(ctx)
    }
    
    // --- POOL ---
    pub fn create_pool(
        ctx: Context<CreatePool>,
        name: String,
        metadata: Option<String>,
        start_time: i64,
        end_time: i64,
        max_accuracy_buffer: u64,
        conviction_bonus_bps: u64,
    ) -> Result<()> {
        pool::create_pool(
            ctx,
            name,
            metadata,
            start_time,
            end_time,
            max_accuracy_buffer,
            conviction_bonus_bps,
        )
    }

    pub fn place_bet(
        ctx: Context<PlaceBet>,
        amount: u64,
        commitment: [u8; 32], 
        request_id: String,
    ) -> Result<()> {
        pool::place_bet(
            ctx,
            amount,
            commitment,
            request_id,
        )
    }

    pub fn resolve_pool(
        ctx: Context<ResolvePool>,
        final_outcome: u64,
    ) -> Result<()> {
        pool::resolve_pool(ctx, final_outcome)
    }

    pub fn calculate_outcome(ctx: Context<CalculateOutcome>) -> Result<()> {
        pool::calculate_outcome(ctx)
    }

    pub fn batch_calculate_outcome<'info>(
        ctx: Context<'_, '_, '_, 'info, BatchCalculateOutcome<'info>>
    ) -> Result<()> {
        admin::batch_calculate_outcome(ctx)
    }

    pub fn finalize_weights(ctx: Context<FinalizeWeights>) -> Result<()> {
        pool::finalize_weights(ctx)
    }

    pub fn claim_reward(ctx: Context<ClaimReward>) -> Result<()> {
        pool::claim_reward(ctx)
    }

    // --- BET MANAGEMENT ---
    pub fn update_bet(
        ctx: Context<UpdateBet>,
        new_prediction_target: u64,
    ) -> Result<()> {
        pool::update_bet(
            ctx,
            new_prediction_target,
        )
    }
    
    pub fn reveal_bet(
        ctx: Context<RevealBet>,
        prediction_target: u64,
        salt: [u8; 32],
    ) -> Result<()> {
        pool::reveal_bet(
            ctx,
            prediction_target,
            salt
        )
    }

    pub fn refund_bet(ctx: Context<RefundBet>) -> Result<()> {
        pool::refund_bet(ctx)
    }

    pub fn emergency_refund(ctx: Context<EmergencyRefund>) -> Result<()> {
        pool::emergency_refund(ctx)
    }
}