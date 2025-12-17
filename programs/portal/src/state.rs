use std::cmp::max;

use anchor_lang::prelude::*;
use anchor_spl::associated_token::spl_associated_token_account::solana_program::keccak::hashv;

#[constant]
pub const GLOBAL_SEED: &[u8] = b"global";
#[constant]
pub const M_VAULT_SEED: &[u8] = b"m_vault";
#[constant]
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";
#[constant]
pub use common::interfaces::AUTHORITY_SEED;
#[constant]
pub const MESSAGE_SEED: &[u8] = b"message";

#[account]
pub struct PortalGlobal {
    pub bump: u8,
    pub chain_id: u32,
    pub admin: Pubkey,
    pub paused: bool,
    pub m_index: u64,
    pub message_nonce: u64,
    pub pending_admin: Option<Pubkey>,
    pub isolated_spokes: Vec<IsolatedSpoke>,
    pub padding: [u8; 128],
}

#[account]
pub struct IsolatedSpoke {
    pub chain_id: u32,
    pub bridged_amount: u128,
}

impl PortalGlobal {
    pub fn size(isolated_spokes: usize) -> usize {
        8 + // discriminator
        1 + // bump
        4 + // chain_id
        32 + // admin
        1 + // paused
        8 + // m_index
        8 + // message_nonce
        1 + // pending_admin option
        32 + // pending_admin pubkey
        4 + // length of isolated_spokes
        isolated_spokes * 20 + // each isolated_spoke
        128 // padding
    }

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

#[account]
#[derive(InitSpace)]
pub struct BridgeMessage {
    pub consumed: bool,
}

impl BridgeMessage {
    pub const SIZE: usize = BridgeMessage::INIT_SPACE + BridgeMessage::DISCRIMINATOR.len();
}
