use anchor_lang::{constant, prelude::Pubkey, pubkey};

#[constant]
pub const SPL_NOOP: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

cfg_if::cfg_if! {
    if #[cfg(feature = "mainnet")] {
        #[constant]
        pub const MAILBOX_PROGRAM_ID: Pubkey = pubkey!("E588QtVUvresuXq2KoNEwAmoifCzYGpRBdHByN9KQMbi");
    } else if #[cfg(feature = "devnet")] {
        #[constant]
        pub const MAILBOX_PROGRAM_ID: Pubkey = pubkey!("75HBBLae3ddeneJVrZeyrDfv6vb7SMC3aCpBucSXS5aR");
    }
}

pub const HANDLE_DISCRIMINATOR: [u8; 8] = [33, 210, 5, 66, 196, 212, 239, 142];
pub const HANDLE_ACCOUNT_METAS_DISCRIMINATOR: [u8; 8] = [194, 141, 30, 82, 241, 41, 169, 52];
