use crate::{pda, wormhole_adapter};
use anchor_lang::prelude::*;

static IDS: [Pubkey; 1] = [wormhole_adapter::ID];

pub const AUTHORITY_SEED: &[u8] = b"authority";

#[derive(Clone)]
pub struct BridgeAdapter;

impl anchor_lang::Ids for BridgeAdapter {
    fn ids() -> &'static [Pubkey] {
        &IDS
    }
}

impl BridgeAdapter {
    pub fn is_authority(authority: &Pubkey) -> bool {
        IDS.iter()
            .map(|id| pda!(&[AUTHORITY_SEED], id))
            .any(|a| a == *authority)
    }
}
