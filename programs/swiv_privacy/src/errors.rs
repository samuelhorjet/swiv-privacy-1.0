use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("Global protocol is paused.")]
    Paused,
    #[msg("Unauthorized admin action.")]
    Unauthorized,
    #[msg("Math operation overflow.")]
    MathOverflow,
    #[msg("Insufficient liquidity in pool.")]
    InsufficientLiquidity,
    #[msg("Bet is already settled.")]
    AlreadySettled,
    #[msg("Bet is already initialized.")]
    BetAlreadyInitialized,
    #[msg("Bet duration is too short.")]
    DurationTooShort,
    #[msg("Invalid asset symbol.")]
    InvalidAsset,
    #[msg("Asset is not whitelisted.")]
    AssetNotWhitelisted,
    #[msg("Seeds do not result in a valid address.")]
    SeedMismatch,
    #[msg("Bet does not match the current pool/asset config")]
    PoolMismatch,
    #[msg("Private Key must be 32 bytes long.")]
    InvalidPrivateKey,
    #[msg("Admin force-settlement is not yet allowed for this bet.")]
    SettlementTooEarly,
    #[msg("Emergency refund timeout has not been met.")]
    TimeoutNotMet,
    #[msg("Bet has not been calculated by the TEE yet.")]
    NotCalculatedYet,
    #[msg("You must wait for the pool to end before undelegating to preserve privacy.")]
    UndelegationTooEarly,
    #[msg("TEE failed to decrypt the prediction.")]
    DecryptionFailed,
}