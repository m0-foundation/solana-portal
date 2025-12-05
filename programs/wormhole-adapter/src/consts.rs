use anchor_lang::constant;
use anchor_lang::{prelude::Pubkey, pubkey};

#[constant]
pub const AUTHORITY_SEED: &[u8] = b"authority";
#[constant]
pub const GUARDIAN_SET_SEED: &[u8] = b"GuardianSet";
#[constant]
pub const EMITTER_SEED: &[u8] = b"emitter";
#[constant]
pub const SEQUENCE_SEED: &[u8] = b"Sequence";
#[constant]
pub const EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";

cfg_if::cfg_if! {
    if #[cfg(feature = "mainnet")] {
        #[constant]
        pub const CORE_BRIDGE_PROGRAM_ID: Pubkey = pubkey!("worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth");
        #[constant]
        pub const CORE_BRIDGE_FEE_COLLECTOR: Pubkey = pubkey!("9bFNrXNb2WTx8fMHXCheaZqkLZ3YCCaiqTftHxeintHy");
        #[constant]
        pub const CORE_BRIDGE_CONFIG: Pubkey = pubkey!("2yVjuQwpsvdsrywzsJJVs9Ueh4zayyo5DYJbBNc3DDpn");
    } else if #[cfg(feature = "devnet")] {
        #[constant]
        pub const CORE_BRIDGE_PROGRAM_ID: Pubkey = pubkey!("3u8hJUVTA4jH1wYAyUur7FFZVQ8H635K3tSHHF4ssjQ5");
        #[constant]
        pub const CORE_BRIDGE_FEE_COLLECTOR: Pubkey = pubkey!("7s3a1ycs16d6SNDumaRtjcoyMaTDZPavzgsmS3uUZYWX");
        #[constant]
        pub const CORE_BRIDGE_CONFIG: Pubkey = pubkey!("6bi4JGDoRwUs9TYBuvoA7dUVyikTJDrJsJU1ew6KVLiu");
    }
}
