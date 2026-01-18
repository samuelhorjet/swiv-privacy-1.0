use anchor_lang::prelude::*;

#[event]
pub struct ProtocolInitialized {
    pub admin: Pubkey,
    pub fee_wallet: Pubkey,
}

#[event]
pub struct ConfigUpdated {
    pub treasury: Option<Pubkey>,
    pub protocol_fee_bps: Option<u64>,
}

#[event]
pub struct BetPlaced {
    pub bet_address: Pubkey,
    pub user: Pubkey,
    pub pool_identifier: String,
    pub amount: u64,
    pub end_timestamp: i64,
}

#[event]
pub struct BetRevealed {
    pub bet_address: Pubkey,
    pub decrypted_target: u64
}

#[event]
pub struct BetUpdated {
    pub bet_address: Pubkey,
    pub user: Pubkey,
}

#[event]
pub struct PoolCreated {
    pub pool_name: String,
    pub start_time: i64,
    pub end_time: i64,
}

#[event]
pub struct AssetConfigUpdated {
    pub symbol: String,
    pub pyth_feed: Pubkey,
    pub volatility_factor: u64,
}

#[event]
pub struct PauseChanged {
    pub is_paused: bool,
}

#[event]
pub struct AdminTransferred {
    pub old_admin: Pubkey,
    pub new_admin: Pubkey,
}

#[event]
pub struct BetDelegated {
    pub bet_address: Pubkey,
    pub user: Pubkey,
    pub request_id: String,
}

#[event]
pub struct BetUndelegated {
    pub bet_address: Pubkey,
    pub user: Pubkey,
    pub is_batch: bool,
}

#[event]
pub struct PoolResolved {
    pub pool_name: String,
    pub final_outcome: u64,
    pub resolution_ts: i64,
}

#[event]
pub struct WeightsFinalized {
    pub pool_name: String,
    pub total_weight: u128,
    pub fee_deducted: u64,
}

#[event]
pub struct OutcomeCalculated {
    pub bet_address: Pubkey,
    pub user: Pubkey,
    pub weight: u128,
}

#[event]
pub struct RewardClaimed {
    pub bet_address: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct BetRefunded {
    pub bet_address: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub is_emergency: bool,
}