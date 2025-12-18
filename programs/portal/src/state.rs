use std::cmp::max;

use anchor_lang::prelude::*;

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
#[derive(InitSpace)]
pub struct PortalGlobal {
    pub bump: u8,
    pub chain_id: u32,
    pub admin: Pubkey,
    pub paused: bool,
    pub m_index: u128,
    pub message_nonce: u64,
    pub pending_admin: Option<Pubkey>,
    pub padding: [u8; 128],
}

impl PortalGlobal {
    pub fn update_index(&mut self, message_id: [u8; 32], new_index: u128) {
        self.m_index = max(new_index, self.m_index);

        emit!(MTokenIndexReceived {
            index: new_index,
            message_id,
        });
    }
}

impl PortalGlobal {
    pub const SIZE: usize = PortalGlobal::INIT_SPACE + PortalGlobal::DISCRIMINATOR.len();
}

#[account]
#[derive(InitSpace)]
pub struct BridgeMessage {
    pub consumed: bool,
}

impl BridgeMessage {
    pub const SIZE: usize = BridgeMessage::INIT_SPACE + BridgeMessage::DISCRIMINATOR.len();
}

#[event]
pub struct MTokenIndexReceived {
    pub index: u128,
    pub message_id: [u8; 32],
}
