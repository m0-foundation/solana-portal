use anchor_lang::prelude::*;
use common::BridgeError;

use crate::instructions::VaaBody;

#[constant]
pub const GLOBAL_SEED: &[u8] = b"global";

#[account]
pub struct WormholeGlobal {
    pub bump: u8,
    pub admin: Pubkey,
    pub paused: bool,
    pub pending_admin: Option<Pubkey>,
    pub peers: Vec<Peer>,
}

#[account]
pub struct Peer {
    pub address: [u8; 32],
    pub chain_id: u32,
}

impl WormholeGlobal {
    pub fn size(peers: usize) -> usize {
        8 + // discriminator
        1 + // bump
        32 + // admin
        1 + // paused
        1 + // pending_admin option
        32 + // pending_admin pubkey
        4 + // length of peers
        peers * 36 // each peer
    }

    pub fn validate(&self, vaa: &VaaBody) -> Result<()> {
        if self
            .peers
            .iter()
            .find(|p| p.chain_id == (vaa.emitter_chain as u32) && p.address == vaa.emitter_address)
            .is_none()
        {
            return err!(BridgeError::InvalidPeer);
        }

        Ok(())
    }

    pub fn get_peer(&self, chain_id: u32) -> Result<Peer> {
        self.peers
            .iter()
            .find(|peer| peer.chain_id == chain_id)
            .cloned()
            .ok_or_else(|| BridgeError::UnsupportedDestinationChain.into())
    }
}
