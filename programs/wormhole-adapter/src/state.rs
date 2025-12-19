use anchor_lang::prelude::*;
use common::{BridgeError, Peers};

use crate::instructions::VaaBody;

#[constant]
pub const GLOBAL_SEED: &[u8] = b"global";

#[account]
pub struct WormholeGlobal {
    pub bump: u8,
    pub admin: Pubkey,
    pub paused: bool,
    pub chain_id: u32,
    pub receive_lut: Option<Pubkey>,
    pub pending_admin: Option<Pubkey>,
    pub peers: Peers,
    pub padding: [u8; 128],
}

impl WormholeGlobal {
    pub fn size(peers: usize) -> usize {
        8 + // discriminator
        1 + // bump
        32 + // admin
        1 + // paused
        4 + // chain_id
        1 + // receive_lut option
        32 + // receive_lut
        1 + // pending_admin option
        32 + // pending_admin pubkey
        Peers::size(peers) +
        128 // padding
    }

    pub fn validate(&self, vaa: &VaaBody) -> Result<()> {
        let peer = self.peers.get_peer(vaa.emitter_chain as u32)?;
        if peer.address != vaa.emitter_address {
            return err!(BridgeError::InvalidPeer);
        }

        Ok(())
    }
}
