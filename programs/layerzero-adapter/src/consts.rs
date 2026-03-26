use anchor_lang::prelude::Pubkey;
use anchor_lang::pubkey;
use anchor_lang::solana_program::hash::hash;

pub const LZ_ENDPOINT_PROGRAM_ID: Pubkey = pubkey!("76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6");

/// LayerZero endpoint PDA seeds
pub const LZ_ENDPOINT_SEED: &[u8] = b"Endpoint";
pub const LZ_OAPP_SEED: &[u8] = b"OApp";
pub const LZ_NONCE_SEED: &[u8] = b"Nonce";
pub const LZ_PAYLOAD_HASH_SEED: &[u8] = b"PayloadHash";
pub const LZ_EVENT_SEED: &[u8] = b"__event_authority";
pub const LZ_SEND_LIBRARY_CONFIG_SEED: &[u8] = b"SendLibraryConfig";
pub const LZ_RECEIVE_LIBRARY_CONFIG_SEED: &[u8] = b"ReceiveLibraryConfig";
pub const LZ_MESSAGE_LIB_SEED: &[u8] = b"MessageLib";

/// Pre-computed Anchor discriminators for LZ endpoint instructions.
/// These are the first 8 bytes of sha256("global:<instruction_name>").
pub const REGISTER_OAPP_DISCRIMINATOR: [u8; 8] = [0x81, 0x59, 0x47, 0x44, 0x0b, 0x52, 0xd2, 0x7d];
pub const SEND_DISCRIMINATOR: [u8; 8] = [0x66, 0xfb, 0x14, 0xbb, 0x41, 0x4b, 0x0c, 0x45];
pub const CLEAR_DISCRIMINATOR: [u8; 8] = [0xfa, 0x27, 0x1c, 0xd5, 0x7b, 0xa3, 0x85, 0x05];
pub const QUOTE_DISCRIMINATOR: [u8; 8] = [0x95, 0x2a, 0x6d, 0xf7, 0x86, 0x92, 0xd5, 0x7b];
pub const SET_DELEGATE_DISCRIMINATOR: [u8; 8] = [0xf2, 0x1e, 0x2e, 0x4c, 0x6c, 0xeb, 0x80, 0xb5];

/// Compute an Anchor discriminator at runtime (for verification tests).
pub fn anchor_discriminator(name: &str) -> [u8; 8] {
    let full = format!("global:{}", name);
    let h = hash(full.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&h.to_bytes()[..8]);
    disc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_discriminators() {
        assert_eq!(
            anchor_discriminator("register_oapp"),
            REGISTER_OAPP_DISCRIMINATOR
        );
        assert_eq!(anchor_discriminator("send"), SEND_DISCRIMINATOR);
        assert_eq!(anchor_discriminator("clear"), CLEAR_DISCRIMINATOR);
        assert_eq!(anchor_discriminator("quote"), QUOTE_DISCRIMINATOR);
        assert_eq!(
            anchor_discriminator("set_delegate"),
            SET_DELEGATE_DISCRIMINATOR
        );
    }
}
