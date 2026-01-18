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
    #[msg("Bet duration is too short.")]
    DurationTooShort,
    #[msg("Invalid asset symbol.")]
    InvalidAsset,
    #[msg("Asset is not whitelisted.")]
    AssetNotWhitelisted,
    #[msg("Bet does not match the current pool/asset config")]
    PoolMismatch,
    #[msg("Oracle price is non-positive.")]
    InvalidOraclePrice,
    #[msg("Admin force-settlement is not yet allowed for this bet.")]
    SettlementTooEarly,
    #[msg("Emergency refund timeout has not been met.")]
    TimeoutNotMet,
    #[msg("Bet has not been calculated by the TEE yet.")]
    NotCalculatedYet,
    #[msg("The provided prediction does not match the commitment hash.")]
    InvalidCommitment,
    #[msg("Bet is already revealed.")]
    AlreadyRevealed,
    #[msg("Bet is not yet revealed.")]
    BetNotRevealed,
    #[msg("You cannot refund a bet that has been revealed. Wait for settlement.")]
    CannotRefundRevealed,
    #[msg("Reveal window has expired. Please request a refund.")]
    RevealWindowExpired,
    #[msg("You must wait for the pool to end before undelegating to preserve privacy.")]
    UndelegationTooEarly,
}