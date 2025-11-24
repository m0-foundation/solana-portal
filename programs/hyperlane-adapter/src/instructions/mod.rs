pub mod initialize;
pub mod receive_message;
pub mod send_message;
pub mod set_peers;
pub mod sync_extensions;

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
pub use initialize::*;
pub use receive_message::*;
pub use send_message::*;
pub use set_peers::*;
pub use sync_extensions::*;

use crate::consts::{MAILBOX_PROGRAM_ID, SPL_NOOP};

pub struct Mailbox;
impl anchor_lang::Id for Mailbox {
    fn id() -> anchor_lang::prelude::Pubkey {
        MAILBOX_PROGRAM_ID
    }
}

pub struct SplNoop;
impl anchor_lang::Id for SplNoop {
    fn id() -> anchor_lang::prelude::Pubkey {
        SPL_NOOP
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct SerializableAccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<AccountMeta> for SerializableAccountMeta {
    fn from(account_meta: AccountMeta) -> Self {
        Self {
            pubkey: account_meta.pubkey,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct SimulationReturnData<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub return_data: T,
    trailing_byte: u8,
}

impl<T> SimulationReturnData<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub fn new(return_data: T) -> Self {
        Self {
            return_data,
            trailing_byte: u8::MAX,
        }
    }
}
