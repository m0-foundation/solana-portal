use anchor_lang::pubkey;
use solana_sdk::pubkey::Pubkey;

// Anchor event CPI discriminator for Wormhole post_message event emitted by shim
pub const WH_EVENT_DISCRIMINATOR: [u8; 8] = [228, 69, 165, 46, 81, 203, 154, 29];

// Wormhole shim PostMessage instruction discriminator (first 8 bytes of data)
pub const WH_SHIM_POST_MESSAGE_DISCRIMINATOR: [u8; 8] = [214, 50, 100, 209, 38, 34, 7, 76];

pub const ETHEREUM_WORMHOLE_TRANSCEIVER: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 99, 25, 106, 9, 21, 117, 173, 249, 158, 35, 6, 229, 233,
    14, 11, 229, 21, 72, 65,
];

pub const SOLANA_CHAIN_ID: u32 = 1399811149;

pub const M_MINT: Pubkey = pubkey!("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH");
