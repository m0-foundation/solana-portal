use anchor_lang::prelude::*;
use anchor_spl::associated_token::spl_associated_token_account::solana_program::hash::hashv;
use common::{Extension, Peers};
use common::{BridgeError, Extension};

#[constant]
pub const GLOBAL_SEED: &[u8] = b"global";
#[constant]
pub const HYPERLANE_SEED: &[u8] = b"hyperlane";
#[constant]
pub const HYPERLANE_IGP_SEED: &[u8] = b"hyperlane_igp";
#[constant]
pub const METADATA_SEED_1: &[u8] = b"hyperlane_message_recipient";
#[constant]
pub const METADATA_SEED_2: &[u8] = b"handle";
#[constant]
pub const METADATA_SEED_3: &[u8] = b"account_metas";
#[constant]
pub const DISPATCH_SEED_1: &[u8] = b"hyperlane_dispatcher";
#[constant]
pub const DISPATCH_SEED_2: &[u8] = b"dispatch_authority";
#[constant]
pub const PROCESS_AUTHORITY: &[u8] = b"process_authority";
#[constant]
pub const OUTBOX_SEED: &[u8] = b"outbox";
#[constant]
pub const DISPATCHED_MESSAGE_SEED: &[u8] = b"dispatched_message";
#[constant]
pub const UNIQUE_MESSAGE_SEED: &[u8] = b"unique_message";
#[constant]
pub const PROGRAM_DATA_SEED: &[u8] = b"program_data";
#[constant]
pub const GAS_PAYMENT_SEED: &[u8] = b"gas_payment";
#[constant]
pub const PAYER_SEED: &[u8] = b"payer";
#[constant]
pub const DASH_SEED: &[u8] = b"-";

#[account]
pub struct HyperlaneGlobal {
    pub bump: u8,
    pub admin: Pubkey,
    pub paused: bool,
    pub chain_id: u32,
    pub igp_program_id: Pubkey,
    pub igp_gas_amount: u64,
    pub igp_account: Pubkey,
    pub igp_overhead_account: Option<Pubkey>,
    pub ism: Option<Pubkey>,
    pub pending_admin: Option<Pubkey>,
    pub peers: Peers,
    pub padding: [u8; 128],
}

impl HyperlaneGlobal {
    pub fn size(peers: usize) -> usize {
        8 + // discriminator
        1 + // bump
        32 + // admin
        1 + // paused
        4 + // chain_id
        32 + // igp program id
        8 + // igp gas amount
        32 + // igp account
        1 + 32 + // igp overhead account option + pubkey
        1 + 32 + // ism option + ism pubkey
        1 + 32 + // pending admin
        Peers::size(peers) + // peers
        128 // padding
    }
}

#[account]
pub struct AccountMetasData {
    pub bump: u8,
    pub m_mint: Pubkey,
    pub extensions: Vec<Extension>,
}

impl AccountMetasData {
    pub fn size(extensions: usize) -> usize {
        8 + // discriminator
        1 + // bump
        32 + // m_mint
        4 + // length of extensions vector
        extensions * Extension::SIZE
    }
}

#[account]
pub struct HyperlaneUserGlobal {
    pub bump: u8,
    pub user: Pubkey,
    pub nonce: u64,
}

impl HyperlaneUserGlobal {
    pub fn size() -> usize {
        8 + // discriminator
        1 + // bump
        32 + // user
        8 // nonce
    }
}
