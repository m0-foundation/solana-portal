// Centralized discriminators and related constants for adapter decoding.

// Anchor event CPI discriminator for Wormhole post_message event emitted by shim
pub const WH_EVENT_DISCRIMINATOR: [u8; 8] = [228, 69, 165, 46, 81, 203, 154, 29];

// Wormhole shim PostMessage instruction discriminator (first 8 bytes of data)
pub const WH_SHIM_POST_MESSAGE_DISCRIMINATOR: [u8; 8] = [214, 50, 100, 209, 38, 34, 7, 76];
