use anchor_lang::{prelude::Pubkey, pubkey};

pub const SPL_NOOP: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

cfg_if::cfg_if! {
    if #[cfg(feature = "mainnet")] {
        pub const MAILBOX_PROGRAM_ID: Pubkey = pubkey!("E588QtVUvresuXq2KoNEwAmoifCzYGpRBdHByN9KQMbi");
        pub const DEFAULT_IGP_PROGRAM_ID: Pubkey = pubkey!("BhNcatUDC2D5JTyeaqrdSukiVFsEHK7e3hVmKMztwefv");
        pub const DEFAULT_IGP_ACCOUNT: Pubkey = pubkey!("JAvHW21tYXE9dtdG83DReqU2b4LUexFuCbtJT5tF8X6M");
        pub const DEFAULT_OVERHEAD_IGP_ACCOUNT: Pubkey = pubkey!("AkeHBbE5JkwVppujCQQ6WuxsVsJtruBAjUo6fDCFp6fF");
    } else if #[cfg(feature = "testnet")] {
        pub const MAILBOX_PROGRAM_ID: Pubkey = pubkey!("75HBBLae3ddeneJVrZeyrDfv6vb7SMC3aCpBucSXS5aR");
        pub const DEFAULT_IGP_PROGRAM_ID: Pubkey = pubkey!("5p7Hii6CJL4xGBYYTGEQmH9LnUSZteFJUu9AVLDExZX2");
        pub const DEFAULT_IGP_ACCOUNT: Pubkey = pubkey!("9SQVtTNsbipdMzumhzi6X8GwojiSMwBfqAhS7FgyTcqy");
        pub const DEFAULT_OVERHEAD_IGP_ACCOUNT: Pubkey = pubkey!("hBHAApi5ZoeCYHqDdCKkCzVKmBdwywdT3hMqe327eZB");
    }
}

pub const HANDLE_DISCRIMINATOR: [u8; 8] = [33, 210, 5, 66, 196, 212, 239, 142];
pub const HANDLE_ACCOUNT_METAS_DISCRIMINATOR: [u8; 8] = [194, 141, 30, 82, 241, 41, 169, 52];
pub const ISM_DISCRIMINATOR: [u8; 8] = [45, 18, 245, 87, 234, 46, 246, 15];
pub const ISM_METAS_DISCRIMINATOR: [u8; 8] = [190, 214, 218, 129, 67, 97, 4, 76];

/// The amount of gas to pay for bridge message
pub const DEFAULT_HANDLE_GAS_AMOUNT: u64 = 50000;
