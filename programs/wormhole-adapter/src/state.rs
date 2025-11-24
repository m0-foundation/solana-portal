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
        4 + // length of peers
        peers * 34 // each peer
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

    pub fn extended_peers(&self, peers: Vec<Peer>) -> Vec<Peer> {
        let mut result = self.peers.clone();
        for peer in peers {
            match result.iter_mut().find(|p| p.chain_id == peer.chain_id) {
                Some(existing_peer) => *existing_peer = peer, // Overwrite existing peer
                None => result.push(peer),
            }
        }
        result
    }
}
