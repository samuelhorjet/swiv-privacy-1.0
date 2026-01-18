use anchor_lang::prelude::*;

#[account]
pub struct Pool {
    pub admin: Pubkey,
    pub name: String,
    pub token_mint: Pubkey,
    
    pub start_time: i64,
    pub end_time: i64,
    pub vault_balance: u64,
    
    pub max_accuracy_buffer: u64,
    pub conviction_bonus_bps: u64, 
    
    pub metadata: Option<String>,

    pub resolution_target: u64,
    pub is_resolved: bool,
    pub resolution_ts: i64,
    
    pub total_weight: u128,     
    pub weight_finalized: bool, 
    
    pub bump: u8,
}