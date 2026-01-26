pub mod evm;
pub use m0_portal_common::{ExecutorTransactions, WormholeResponse};
use solana_sdk::hash::hashv;

pub fn calculate_instruction_discriminator(instruction_name: &str) -> [u8; 8] {
    let seed = format!("global:{}", instruction_name);
    let hash = hashv(&[seed.as_bytes()]);
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.as_ref()[0..8]);
    discriminator
}
