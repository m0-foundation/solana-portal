pub mod enable_cross_spoke_transfers;
pub mod initialize;
pub mod pause;
pub mod receive_message;
pub mod send_fill_report;
pub mod send_index;
pub mod send_merkle_root;
pub mod send_token;
pub mod transfer_admin;

use anchor_lang::prelude::*;
use common::{hyperlane_adapter, wormhole_adapter, BridgeError, PayloadData};

pub use enable_cross_spoke_transfers::*;
pub use initialize::*;
pub use pause::*;
pub use receive_message::*;
pub use send_fill_report::*;
pub use send_index::*;
pub use send_merkle_root::*;
pub use send_token::*;
pub use transfer_admin::*;

use crate::state::AUTHORITY_SEED;

pub fn send_message<'info>(
    bridge_adapter: AccountInfo<'info>,
    sender: AccountInfo<'info>,
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
            payload.encode(),
            payload_type,
        )
    } else if bridge_adapter.key() == common::hyperlane_adapter::ID {
        if remaining_accounts.len() < 12 {
            return err!(BridgeError::InvalidRemainingAccounts);
        }

        // Delegate account validation to hyperlane adapter
        let hyperlane_global = remaining_accounts[0].clone();
        let mailbox_outbox = remaining_accounts[1].clone();
        let dispatch_authority = remaining_accounts[2].clone();
        let hyperlane_user_global = remaining_accounts[3].clone();
        let unique_message = remaining_accounts[4].clone();
        let dispatched_message = remaining_accounts[5].clone();
        let igp_program_id = remaining_accounts[6].clone();
        let igp_program_data = remaining_accounts[7].clone();
        let igp_gas_payment = remaining_accounts[8].clone();
        let igp_account = remaining_accounts[9].clone();
        let mailbox_program = remaining_accounts[10].clone();
        let spl_noop_program = remaining_accounts[11].clone();

        // Account is optional
        let igp_overhead_account = remaining_accounts.get(12).cloned();

        hyperlane_adapter::cpi::send_message(
            CpiContext::new_with_signer(
                bridge_adapter.to_account_info(),
                hyperlane_adapter::cpi::accounts::SendMessage {
                    payer: sender,
                    hyperlane_global,
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
            payload.encode(),
            payload_type,
        )
    } else {
        err!(BridgeError::InvalidBridgeAdapter)
    }
}
