use anchor_lang::prelude::*;

use crate::{
    hyperlane_adapter::{self},
    pda, portal,
    wormhole_adapter::{self},
};

static IDS: [Pubkey; 2] = [wormhole_adapter::ID, hyperlane_adapter::ID];

pub const AUTHORITY_SEED: &[u8] = b"authority";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum BridgeAdapter {
    Hyperlane,
    Wormhole,
}

impl anchor_lang::Ids for BridgeAdapter {
    fn ids() -> &'static [Pubkey] {
        &IDS
    }
}

impl BridgeAdapter {
    pub fn program_id(&self) -> Pubkey {
        match self {
            BridgeAdapter::Hyperlane => hyperlane_adapter::ID,
            BridgeAdapter::Wormhole => wormhole_adapter::ID,
        }
    }

    pub fn get_id(&self) -> Pubkey {
        self.program_id()
    }

    pub fn authority(&self) -> Pubkey {
        pda!(&[AUTHORITY_SEED], &self.program_id())
    }

    pub fn from_authority(authority: &Pubkey) -> Option<Self> {
        if *authority == Self::Hyperlane.authority() {
            Some(Self::Hyperlane)
        } else if *authority == Self::Wormhole.authority() {
            Some(Self::Wormhole)
        } else {
            None
        }
    }

    pub fn valid_destination_peer(address: [u8; 32]) -> bool {
        [wormhole_adapter::ID, hyperlane_adapter::ID, portal::ID]
            .iter()
            .any(|id| id.to_bytes() == address)
    }
}
