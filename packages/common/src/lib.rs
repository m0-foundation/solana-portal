pub mod accounts;
pub mod conversions;
pub mod errors;
pub mod interfaces;
pub mod payloads;

pub use accounts::*;
use anchor_lang::prelude::*;
pub use conversions::*;
pub use errors::*;
pub use interfaces::*;
pub use payloads::*;

declare_program!(wormhole_post_message_shim);
declare_program!(portal);
declare_program!(wormhole_verify_vaa_shim);
declare_program!(earn);
declare_program!(ext_swap);
declare_program!(order_book);
declare_program!(wormhole_adapter);
declare_program!(hyperlane_adapter);

#[macro_export]
macro_rules! pda {
    ($seeds:expr, $program_id:expr) => {
        Pubkey::find_program_address($seeds, $program_id).0
    };
}
