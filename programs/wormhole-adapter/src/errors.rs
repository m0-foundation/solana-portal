use anchor_lang::prelude::*;

#[error_code]
pub enum WormholeError {
    Paused,
    #[msg("Invalid peer address or chain")]
    InvalidPeer,
    InvalidVaa,
    #[msg("RESOLVER_RESULT_ACCOUNT needs to be writable")]
    InvalidReturnAccount,
    MissingPayerAccount,
    InvalidSwapConfig,
}
