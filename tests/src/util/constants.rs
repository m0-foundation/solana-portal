use anchor_lang::pubkey;
use solana_sdk::pubkey::Pubkey;

// Anchor event CPI discriminator for Wormhole post_message event emitted by shim
pub const WH_EVENT_DISCRIMINATOR: [u8; 8] = [228, 69, 165, 46, 81, 203, 154, 29];

// Wormhole shim PostMessage instruction discriminator (first 8 bytes of data)
pub const WH_SHIM_POST_MESSAGE_DISCRIMINATOR: [u8; 8] = [214, 50, 100, 209, 38, 34, 7, 76];

pub const ETHEREUM_WORMHOLE_ADAPTER: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 107, 42, 123, 250, 95, 28, 3, 235, 250, 231, 121, 223, 105,
    136, 184, 172, 20, 202, 65, 85,
];

pub const ETHEREUM_HYPERLANE_ADAPTER: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 252, 68, 218, 221, 117, 138, 119, 55, 172, 146, 0, 5, 158,
    159, 205, 21, 33, 215, 90, 7,
];

pub const SOLANA_CHAIN_ID: u32 = 1399811149;

pub const M_MINT: Pubkey = pubkey!("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH");
