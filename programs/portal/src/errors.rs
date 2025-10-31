use anchor_lang::prelude::*;

#[error_code]
pub enum PortalError {
    Paused,
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
}
