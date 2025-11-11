use anchor_lang::{prelude::Pubkey, pubkey};

pub const SPL_NOOP: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

cfg_if::cfg_if! {
    if #[cfg(feature = "mainnet")] {
        pub const MAILBOX_PROGRAM_ID: Pubkey = pubkey!("E588QtVUvresuXq2KoNEwAmoifCzYGpRBdHByN9KQMbi");
    } else if #[cfg(feature = "devnet")] {
        pub const MAILBOX_PROGRAM_ID: Pubkey = pubkey!("75HBBLae3ddeneJVrZeyrDfv6vb7SMC3aCpBucSXS5aR");
    }
}
