use std::cmp::max;

use anchor_lang::prelude::*;
use anchor_spl::associated_token::spl_associated_token_account::solana_program::keccak::hashv;

#[constant]
pub const GLOBAL_SEED: &[u8] = b"global";

#[constant]
pub use common::interfaces::AUTHORITY_SEED;

#[account]
#[derive(InitSpace)]
pub struct PortalGlobal {
    pub bump: u8,
    pub chain_id: u32,
    pub admin: Pubkey,
    pub paused: bool,
    pub m_index: u64,
    pub message_nonce: u64,
    pub pending_admin: Option<Pubkey>,
}

impl PortalGlobal {
    pub fn update_index(&mut self, new_index: u64) {
        self.m_index = max(new_index, self.m_index);
    }

    pub fn generate_message_id(&mut self) -> [u8; 32] {
        self.message_nonce += 1;
        hashv(&[
            &self.chain_id.to_le_bytes(),
            &self.message_nonce.to_le_bytes(),
        ])
        .to_bytes()
    }
}

impl PortalGlobal {
    pub const SIZE: usize = PortalGlobal::INIT_SPACE + PortalGlobal::DISCRIMINATOR.len();
}
