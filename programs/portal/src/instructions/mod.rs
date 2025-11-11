pub mod initialize;
pub mod receive_message;
pub mod send_fill_report;
pub mod send_token;

use anchor_lang::prelude::*;
use common::{hyperlane_adapter, wormhole_adapter};
pub use initialize::*;
pub use receive_message::*;
pub use send_fill_report::*;
pub use send_token::*;

use crate::{errors::PortalError, state::AUTHORITY_SEED};

pub fn send_message<'info>(
    bridge_adapter: AccountInfo<'info>,
    sender: AccountInfo<'info>,
    messenger_authority: AccountInfo<'info>,
    messenger_authority_bump: u8,
    system_program: AccountInfo<'info>,
    remaining_accounts: Vec<AccountInfo<'info>>,
    message: Vec<u8>,
    destination_chain_id: u16,
) -> Result<()> {
    // Send the bridge message based on provided adapter
    if bridge_adapter.key() == wormhole_adapter::ID {
        // Delegate account validation to wormhole adapter
        let [
                wormhole_global,
                bridge,
                message_account,
                emitter,
                sequence,
                fee_collector,
                clock,
                wormhole_program,
                wormhole_post_message_shim_ea,
                wormhole_post_message_shim
            ]: [AccountInfo; 10] = remaining_accounts
                .try_into()
                .map_err(|_| PortalError::InvalidRemainingAccounts)?;

        wormhole_adapter::cpi::send_message(
            CpiContext::new_with_signer(
                bridge_adapter.to_account_info(),
                wormhole_adapter::cpi::accounts::SendMessage {
                    payer: sender,
                    wormhole_global,
                    messenger_authority,
                    bridge,
                    message: message_account,
                    emitter,
                    sequence,
                    fee_collector,
                    clock,
                    system_program,
                    wormhole_program,
                    wormhole_post_message_shim_ea,
                    wormhole_post_message_shim,
                },
                &[&[AUTHORITY_SEED, &[messenger_authority_bump]]],
            ),
            message,
        )
    } else if bridge_adapter.key() == common::hyperlane_adapter::ID {
        // Delegate account validation to hyperlane adapter
        let [
                hyperlane_global,
                mailbox_outbox,
                dispatch_authority,
                unique_message,
                dispatched_message,
                mailbox_program,
                spl_noop_program,
            ]: [AccountInfo; 7] = remaining_accounts
                .try_into()
                .map_err(|_| PortalError::InvalidRemainingAccounts)?;

        hyperlane_adapter::cpi::send_message(
            CpiContext::new_with_signer(
                bridge_adapter.to_account_info(),
                hyperlane_adapter::cpi::accounts::SendMessage {
                    payer: sender,
                    hyperlane_global,
                    messenger_authority,
                    mailbox_outbox,
                    dispatch_authority,
                    unique_message,
                    dispatched_message,
                    mailbox_program,
                    spl_noop_program,
                    system_program,
                },
                &[&[AUTHORITY_SEED, &[messenger_authority_bump]]],
            ),
            message,
            destination_chain_id,
        )
    } else {
        err!(PortalError::InvalidBridgeAdapter)
    }
}
