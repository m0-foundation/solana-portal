use anchor_lang::pubkey;
use solana_sdk::pubkey::Pubkey;

// Anchor event CPI discriminator for Wormhole post_message event emitted by shim
pub const WH_EVENT_DISCRIMINATOR: [u8; 8] = [228, 69, 165, 46, 81, 203, 154, 29];

// Wormhole shim PostMessage instruction discriminator (first 8 bytes of data)
pub const WH_SHIM_POST_MESSAGE_DISCRIMINATOR: [u8; 8] = [214, 50, 100, 209, 38, 34, 7, 76];

pub const ETHEREUM_WORMHOLE_ADAPTER: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 234, 174, 73, 107, 205, 169, 60, 204, 211, 253, 111, 246,
    9, 99, 71, 151, 158, 135, 177, 83,
];

pub const ETHEREUM_HYPERLANE_ADAPTER: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 143, 110, 123, 222, 86, 52, 22, 15, 218, 97, 185, 69,
    220, 159, 65, 185, 101, 228, 6,
];

pub const SOLANA_CHAIN_ID: u32 = 1399811149;

pub const M_MINT: Pubkey = pubkey!("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH");
