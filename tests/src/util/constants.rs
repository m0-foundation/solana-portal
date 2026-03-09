use anchor_lang::pubkey;
use solana_sdk::pubkey::Pubkey;

// Anchor event CPI discriminator for Wormhole post_message event emitted by shim
pub const WH_EVENT_DISCRIMINATOR: [u8; 8] = [228, 69, 165, 46, 81, 203, 154, 29];

// Wormhole shim PostMessage instruction discriminator (first 8 bytes of data)
pub const WH_SHIM_POST_MESSAGE_DISCRIMINATOR: [u8; 8] = [214, 50, 100, 209, 38, 34, 7, 76];

pub const ETHEREUM_WORMHOLE_ADAPTER: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 172, 255, 236, 40, 196, 238, 226, 28, 136, 154, 78, 108, 7,
    4, 197, 64, 237, 157, 79, 221,
];

pub const ETHEREUM_HYPERLANE_ADAPTER: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 252, 193, 213, 150, 173, 108, 171, 11, 83, 148, 234, 164,
    71, 216, 98, 104, 19, 24, 15, 50,
];

pub const SOLANA_CHAIN_ID: u32 = 1399811149;

pub const M_MINT: Pubkey = pubkey!("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH");
