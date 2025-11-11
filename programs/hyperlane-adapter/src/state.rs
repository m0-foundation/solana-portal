use anchor_lang::prelude::*;
use common::BridgeError;

#[constant]
pub const GLOBAL_SEED: &[u8] = b"global";

#[account]
pub struct HyperlaneGlobal {
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

impl HyperlaneGlobal {
    pub fn size(peers: usize) -> usize {
        8 + // discriminator
        1 + // bump
        32 + // admin
        1 + // paused
        4 + // length of peers
        peers * 34 // each peer
    }

    pub fn get_peer_by_chain_id(&self, chain_id: u32) -> Result<Peer> {
        self.peers
            .iter()
            .find(|peer| peer.chain_id == chain_id)
            .cloned()
            .ok_or_else(|| BridgeError::UnsupportedDestinationChain.into())
    }
}
