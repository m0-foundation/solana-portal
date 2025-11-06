use anchor_lang::prelude::*;
use crate::wormhole_adapter;

static IDS: [Pubkey; 1] = [wormhole_adapter::ID];

pub const AUTHORITY_SEED: &[u8] = b"authority";
static AUTHORITIES: [Pubkey; 1] = [
    pubkey!("BXYLToEDjKGjmGC2qPNPtDZqfq4topR9Lro1q31jVmd4"), // calculated as pda!(&[AUTHORITY_SEED], &wormhole_adapter::ID),
];

#[derive(Clone)]
pub struct BridgeAdapter;

impl anchor_lang::Ids for BridgeAdapter {
    fn ids() -> &'static [Pubkey] {
        &IDS
    }
}

impl BridgeAdapter {
    pub fn authorities() -> &'static [Pubkey] {
        &AUTHORITIES
    }

    pub fn is_authority(authority: &Pubkey) -> bool {
        Self::authorities().iter().any(|a| a == authority)
    }
}