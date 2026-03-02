use anchor_lang::prelude::*;

#[account]
pub struct Pool {
    pub created_by: Pubkey,
    pub title: String,
    pub pool_id: u64,
    pub stake_token_mint: Pubkey,
    
    pub start_time: i64,
    pub end_time: i64,
    pub total_volume: u64,
    
    pub max_accuracy_buffer: u64,
    pub conviction_bonus_bps: u64, 

    pub resolution_result: u64,
    pub is_resolved: bool,
    pub resolution_ts: i64,
    
    pub total_weight: u128,     
    pub weight_finalized: bool,
    pub total_participants: u64,
    
    pub bump: u8,
}