use anchor_lang::prelude::*;

#[account]
pub struct Protocol {
    pub admin: Pubkey,
    pub treasury_wallet: Pubkey,
    pub protocol_fee_bps: u64, 
    pub paused: bool,
    pub batch_settle_wait_duration: i64,
    pub total_pools: u64,
}

impl Protocol {
    pub const BASE_LEN: usize = 8 + 32 + 32 + 8 + 1 + 8 + 8;
}
