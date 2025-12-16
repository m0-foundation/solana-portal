pub mod constants;
pub mod hyperlane;
pub mod wormhole;

use solana_sdk::keccak::hashv;

pub fn compute_expected_message_id(chain_id: u32, message_nonce: u64) -> [u8; 32] {
    let chain_id_le = chain_id.to_le_bytes();
    let nonce_le = message_nonce.to_le_bytes();
    hashv(&[&chain_id_le[..], &nonce_le[..]]).to_bytes()
}
