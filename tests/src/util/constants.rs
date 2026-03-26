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

// Placeholder Ethereum LayerZero adapter address for tests
pub const ETHEREUM_LAYERZERO_ADAPTER: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22, 0x33,
    0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0x00, 0xAB, 0xCD, 0xEF, 0x01,
];

// LayerZero Ethereum EID (used as adapter_chain_id in peer registry)
pub const ETHEREUM_LZ_EID: u32 = 30101;

pub const SOLANA_CHAIN_ID: u32 = 1399811149;

pub const M_MINT: Pubkey = pubkey!("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH");
