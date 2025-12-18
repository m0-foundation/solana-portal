pub mod constants;
pub mod hyperlane;
pub mod wormhole;

pub mod tokens;

use solana_sdk::keccak::hashv;

pub fn compute_expected_message_id(destination_chain_id: u32, chain_id: u32, message_nonce: u64) -> [u8; 32] {
    let destination_chain_id_be = destination_chain_id.to_be_bytes();
    let chain_id_be = chain_id.to_be_bytes();
    let nonce_be = message_nonce.to_be_bytes();
    hashv(&[&destination_chain_id_be[..], &chain_id_be[..], &nonce_be[..]]).to_bytes()
}
