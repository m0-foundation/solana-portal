pub mod bridge_path;
pub mod enable_cross_spoke_transfers;
pub mod initialize;
pub mod pause;
pub mod receive_message;
pub mod send_index;
pub mod send_merkle_root;
pub mod send_report;
pub mod send_token;
pub mod transfer_admin;

use anchor_lang::prelude::*;
use m0_portal_common::{hyperlane_adapter, wormhole_adapter, BridgeError, PayloadData};

pub use bridge_path::*;
pub use enable_cross_spoke_transfers::*;
pub use initialize::*;
pub use pause::*;
pub use receive_message::*;
pub use send_index::*;
pub use send_merkle_root::*;
pub use send_report::*;
pub use send_token::*;
pub use transfer_admin::*;

use crate::state::{PortalGlobal, AUTHORITY_SEED};

pub fn send_message<'info>(
    bridge_adapter: AccountInfo<'info>,
    sender: AccountInfo<'info>,
    portal_global: &mut Account<'info, PortalGlobal>,
    portal_authority: AccountInfo<'info>,
    portal_authority_bump: u8,
    system_program: AccountInfo<'info>,
    remaining_accounts: Vec<AccountInfo<'info>>,
    destination_chain_id: u32,
    payload: PayloadData,
    payload_type: u8,
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
                .map_err(|_| BridgeError::InvalidRemainingAccounts)?;

        wormhole_adapter::cpi::send_message(
            CpiContext::new_with_signer(
                bridge_adapter.to_account_info(),
                wormhole_adapter::cpi::accounts::SendMessage {
                    payer: sender,
                    wormhole_global,
                    portal_global: portal_global.to_account_info(),
                    portal_authority,
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
                &[&[AUTHORITY_SEED, &[portal_authority_bump]]],
            ),
            destination_chain_id,
            portal_global.generate_message_id(destination_chain_id),
            payload.encode(),
            payload_type,
        )
    } else if bridge_adapter.key() == m0_portal_common::hyperlane_adapter::ID {
        if remaining_accounts.len() < 12 {
            return err!(BridgeError::InvalidRemainingAccounts);
        }

        // Delegate account validation to hyperlane adapter
        let [
            hyperlane_global,
            mailbox_outbox,
            dispatch_authority,
            hyperlane_user_global,
            unique_message,
            dispatched_message,
            igp_program_id,
            igp_program_data,
            igp_gas_payment,
            igp_account,
            mailbox_program,
            spl_noop_program,
        ]: [AccountInfo; 12] = remaining_accounts[..12]
            .to_vec()
            .try_into()
            .map_err(|_| BridgeError::InvalidRemainingAccounts)?;

        // Account at index 12 is optional
        let igp_overhead_account = remaining_accounts.get(12).cloned();

        hyperlane_adapter::cpi::send_message(
            CpiContext::new_with_signer(
                bridge_adapter.to_account_info(),
                hyperlane_adapter::cpi::accounts::SendMessage {
                    payer: sender,
                    hyperlane_global,
                    portal_global: portal_global.to_account_info(),
                    portal_authority,
                    mailbox_outbox,
                    dispatch_authority,
                    hyperlane_user_global,
                    unique_message,
                    dispatched_message,
                    mailbox_program,
                    spl_noop_program,
                    system_program,
                    igp_program_id,
                    igp_program_data,
                    igp_gas_payment,
                    igp_account,
                    igp_overhead_account,
                },
                &[&[AUTHORITY_SEED, &[portal_authority_bump]]],
            ),
            destination_chain_id,
            portal_global.generate_message_id(destination_chain_id),
            payload.encode(),
            payload_type,
        )
    } else {
        err!(BridgeError::InvalidBridgeAdapter)
    }
}
