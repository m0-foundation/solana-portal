pub mod accounts;
pub mod conversions;
pub mod payloads;

pub use accounts::*;
use anchor_lang::prelude::*;
pub use conversions::*;
pub use payloads::*;

#[macro_export]
macro_rules! pda {
    ($seeds:expr, $program_id:expr) => {
        Pubkey::find_program_address($seeds, $program_id).0
    };
}

#[error_code]
pub enum CommonError {
    #[msg("Missing optional account required for payload type")]
    MissingOptionalAccount,
    #[msg("Remaining account invalid")]
    InvalidRemainingAccount,
}
