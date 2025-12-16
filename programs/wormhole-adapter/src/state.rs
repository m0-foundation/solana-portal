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
    pub receive_lut: Option<Pubkey>,
    pub pending_admin: Option<Pubkey>,
    pub peers: Vec<Peer>,
    pub padding: [u8; 128],
}

#[account]
pub struct Peer {
    pub address: [u8; 32],
    pub m0_chain_id: u32,
    pub wormhole_chain_id: u32,
}

impl WormholeGlobal {
    pub fn size(peers: usize) -> usize {
        8 + // discriminator
        1 + // bump
        32 + // admin
        1 + // paused
        1 + // receive_lut option
        32 + // receive_lut
        1 + // pending_admin option
        32 + // pending_admin pubkey
        4 + // length of peers
        peers * 40 + // each peer
        128 // padding
    }

    pub fn validate(&self, vaa: &VaaBody) -> Result<()> {
        if self
            .peers
            .iter()
            .find(|p| {
                p.wormhole_chain_id == (vaa.emitter_chain as u32)
                    && p.address == vaa.emitter_address
            })
            .is_none()
        {
            return err!(BridgeError::InvalidPeer);
        }

        Ok(())
    }

    pub fn get_m0_peer(&self, m0_chain_id: u32) -> Result<Peer> {
        self.peers
            .iter()
            .find(|peer| peer.m0_chain_id == m0_chain_id)
            .cloned()
            .ok_or_else(|| BridgeError::UnsupportedDestinationChain.into())
    }

    pub fn updated_peers(&self, peer: Peer) -> Vec<Peer> {
        let mut peers = self.peers.clone();

        // Remove any entries with matching m0_chain_id or wormhole_chain_id
        peers.retain(|p| {
            p.m0_chain_id != peer.m0_chain_id && p.wormhole_chain_id != peer.wormhole_chain_id
        });

        // Only add the new peer if address is set
        if peer.address != [0u8; 32] {
            peers.push(peer);
        }

        peers
    }
}
