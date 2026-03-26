use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use m0_portal_common::{
    portal::{self, accounts::PortalGlobal},
    BridgeError, Payload, PayloadData, PayloadHeader, AUTHORITY_SEED,
};

use crate::{
    consts::SEND_DISCRIMINATOR,
    state::{LayerZeroGlobal, SendParams, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct SendMessage<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump = lz_global.bump,
        constraint = !lz_global.outgoing_paused @ BridgeError::Paused,
    )]
    pub lz_global: Account<'info, LayerZeroGlobal>,

    #[account(
        seeds = [GLOBAL_SEED],
        seeds::program = portal::ID,
        bump = portal_global.bump,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    #[account(
        seeds = [AUTHORITY_SEED],
        seeds::program = portal::ID,
        bump
    )]
    /// Only relay messages coming from the Portal
    pub portal_authority: Signer<'info>,

    /// CHECK: Validated against lz_global.endpoint_program
    #[account(
        constraint = endpoint_program.key() == lz_global.endpoint_program @ BridgeError::InvalidBridgeAdapter,
        executable,
    )]
    pub endpoint_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> SendMessage<'info> {
    pub fn handler(
        ctx: Context<'_, '_, '_, 'info, Self>,
        m0_destination_chain_id: u32,
        message_id: [u8; 32],
        payload: Vec<u8>,
        payload_type: u8,
    ) -> Result<()> {
        let peer = ctx
            .accounts
            .lz_global
            .peers
            .get_m0_peer(m0_destination_chain_id)?
            .clone();

        let message = Payload {
            header: PayloadHeader {
                message_id,
                destination_chain_id: m0_destination_chain_id,
                destination_peer: peer.address,
                payload_type,
                index: ctx.accounts.portal_global.m_index,
            },
            data: PayloadData::decode(payload_type, &payload)?,
        }
        .encode();

        // The remaining_accounts contain all the LZ endpoint send accounts.
        // We forward them directly to the endpoint's send instruction.
        let remaining = ctx.remaining_accounts;

        // Build account metas from remaining_accounts.
        // The first account is the endpoint itself (already known), then the OApp sender
        // (lz_global, signing), then the rest are forwarded from remaining_accounts.
        let lz_global_key = ctx.accounts.lz_global.key();
        let endpoint_key = ctx.accounts.endpoint_program.key();

        let mut accounts = Vec::with_capacity(remaining.len() + 2);
        accounts.push(AccountMeta::new_readonly(lz_global_key, true)); // sender/OApp
        for account in remaining.iter() {
            if account.is_writable {
                accounts.push(AccountMeta::new(account.key(), account.is_signer));
            } else {
                accounts.push(AccountMeta::new_readonly(account.key(), account.is_signer));
            }
        }

        // Default options: empty means the endpoint uses defaults.
        // Callers can encode LZ options into the remaining_accounts flow
        // or we use an empty options Vec for basic sends.
        let send_params = SendParams {
            dst_eid: peer.adapter_chain_id,
            receiver: peer.address,
            message,
            options: vec![],
            native_fee: ctx.accounts.payer.lamports(),
            lz_token_fee: 0,
        };

        let mut instruction_data = Vec::new();
        instruction_data.extend_from_slice(&SEND_DISCRIMINATOR);
        instruction_data.extend_from_slice(&send_params.try_to_vec()?);

        let send_ix = Instruction {
            program_id: endpoint_key,
            data: instruction_data,
            accounts,
        };

        let mut account_infos = Vec::with_capacity(remaining.len() + 2);
        account_infos.push(ctx.accounts.lz_global.to_account_info());
        for account in remaining.iter() {
            account_infos.push(account.to_account_info());
        }

        let bump = ctx.accounts.lz_global.bump;
        invoke_signed(
            &send_ix,
            &account_infos,
            &[&[GLOBAL_SEED, &[bump]]],
        )?;

        Ok(())
    }
}
