use anchor_lang::{AnchorDeserialize, AnchorSerialize, Result};

use crate::BridgeError;

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct Peer {
    pub address: [u8; 32],
    pub m0_chain_id: u32,
    pub adapter_chain_id: u32,
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct Peers(pub Vec<Peer>);

impl Peers {
    pub fn get_m0_peer(&self, m0_chain_id: u32) -> Result<&Peer> {
        self.0
            .iter()
            .find(|peer| peer.m0_chain_id == m0_chain_id)
            .ok_or_else(|| BridgeError::InvalidPeer.into())
    }

    pub fn get_peer(&self, adapter_chain_id: u32) -> Result<&Peer> {
        self.0
            .iter()
            .find(|peer| peer.adapter_chain_id == adapter_chain_id)
            .ok_or_else(|| BridgeError::InvalidPeer.into())
    }

    pub fn updated_peers(&self, peer: Peer) -> Peers {
        let mut peers = self.clone();

        // Remove any entries with matching m0_chain_id or adapter_chain_id
        peers.0.retain(|p| {
            p.m0_chain_id != peer.m0_chain_id && p.adapter_chain_id != peer.adapter_chain_id
        });

        // Only add the new peer if address is set
        if peer.address != [0u8; 32] {
            peers.0.push(peer);
        }

        peers
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn size(peers: usize) -> usize {
        4 + // length prefix
        peers * 40 // each peer
    }
}

impl Default for Peers {
    fn default() -> Self {
        Peers(Vec::new())
    }
}
