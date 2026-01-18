use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum BetStatus {
    Active,
    Calculated, 
    Settled,    
}

#[account]
pub struct UserBet {
    pub owner: Pubkey,
    pub pool_identifier: String,
    
    pub deposit: u64,
    pub end_timestamp: i64,
    
    pub creation_ts: i64,       
    pub update_count: u32,     
    
    pub calculated_weight: u128, 
    pub is_weight_added: bool,


    pub referrer: Option<Pubkey>,
    
    pub commitment: [u8; 32], 
    pub is_revealed: bool,    
    
    pub prediction_target: u64, 
    
    pub status: BetStatus,
    
    pub bump: u8,
}

impl UserBet {
    pub const SPACE: usize = 250; 
}