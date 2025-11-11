pub mod initialize;
pub mod receive_message;
pub mod send_message;

pub use initialize::*;
pub use receive_message::*;
pub use send_message::*;

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
