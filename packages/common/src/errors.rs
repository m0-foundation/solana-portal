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
    #[msg("Missing Wormhole guardian account")]
    MissingGuardianAccount,
    #[msg("Signer is not authorized to perform this action")]
    NotAuthorized,
    InvalidAmount,
    InvalidMint,
    InvalidExtension,
    #[msg("Bridge adapter not supported")]
    InvalidBridgeAdapter,
    #[msg("Account marked as optional is required")]
    MissingRequiredOptional,
    #[msg("Invalid number of remaining accounts")]
    InvalidRemainingAccounts,
    InvalidRecipientTokenAccount,
    #[msg("Expected authority from a supported adapter")]
    InvalidAdapterAuthority,
    #[msg("Invalid Hyperlane IGP account")]
    InvalidIgpAccount,
    #[msg("Message ID does not match payload")]
    InvalidMessageId,
}
