use anchor_lang::prelude::*;

#[error_code]
pub enum BridgeError {
    #[msg("Missing optional account required for payload type")]
    MissingOptionalAccount,
    #[msg("Remaining account invalid")]
    InvalidRemainingAccount,
    Paused,
    #[msg("Invalid peer address or chain")]
    InvalidPeer,
    InvalidVaa,
    #[msg("RESOLVER_RESULT_ACCOUNT needs to be writable")]
    InvalidReturnAccount,
    MissingPayerAccount,
    InvalidSwapConfig,
    #[msg("No registered peer for destination chain")]
    UnsupportedDestinationChain,
}
