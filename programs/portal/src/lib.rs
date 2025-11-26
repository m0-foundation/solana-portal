#![allow(unexpected_cfgs)]

pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;
use instructions::*;

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "M Portal V2 Program",
    project_url: "https://m0.org/",
    contacts: "email:security@m0.xyz",
    policy: "https://github.com/m0-foundation/solana-portal/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/m0-foundation/solana-portal/tree/main/programs/portal",
    auditors: ""
}

declare_id!("MzBrgc8yXBj4P16GTkcSyDZkEQZB9qDqf3fh9bByJce");

#[program]
pub mod portal {
    use super::*;

    /// Admin Instructions

    pub fn initialize(ctx: Context<Initialize>, chain_id: u32) -> Result<()> {
        Initialize::handler(ctx, chain_id)
    }

    pub fn pause(ctx: Context<Pause>) -> Result<()> {
        Pause::handler(ctx)
    }

    pub fn unpause(ctx: Context<Unpause>) -> Result<()> {
        Unpause::handler(ctx)
    }

    pub fn propose_admin(ctx: Context<ProposeAdmin>, new_admin: Pubkey) -> Result<()> {
        ProposeAdmin::handler(ctx, new_admin)
    }

    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        AcceptAdmin::handler(ctx)
    }

    pub fn cancel_admin_transfer(ctx: Context<CancelAdminTransfer>) -> Result<()> {
        CancelAdminTransfer::handler(ctx)
    }

    /// Outbound Instructions

    pub fn send_index<'info>(
        ctx: Context<'_, '_, '_, 'info, SendIndex<'info>>,
        destination_chain_id: u32,
    ) -> Result<()> {
        SendIndex::handler(ctx, destination_chain_id)
    }

    pub fn send_merkle_root<'info>(
        ctx: Context<'_, '_, '_, 'info, SendMerkleRoot<'info>>,
        destination_chain_id: u32,
    ) -> Result<()> {
        SendMerkleRoot::handler(ctx, destination_chain_id)
    }

    pub fn send_token<'info>(
        ctx: Context<'_, '_, '_, 'info, SendTokens<'info>>,
        amount: u64,
        destination_token: [u8; 32],
        destination_chain_id: u32,
        recipient: [u8; 32],
    ) -> Result<()> {
        SendTokens::handler(
            ctx,
            amount,
            destination_token,
            destination_chain_id,
            recipient,
        )
    }

    pub fn send_fill_report<'info>(
        ctx: Context<'_, '_, '_, 'info, SendFillReport<'info>>,
        order_id: [u8; 32],
        token_in: [u8; 32],
        amount_in_to_release: u128,
        amount_out_filled: u128,
        origin_recipient: [u8; 32],
        origin_chain_id: u32,
    ) -> Result<()> {
        SendFillReport::handler(
            ctx,
            order_id,
            token_in,
            amount_in_to_release,
            amount_out_filled,
            origin_recipient,
            origin_chain_id,
        )
    }

    /// Inbound Instructions

    pub fn receive_message<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        payload: Vec<u8>,
    ) -> Result<()> {
        ReceiveMessage::handler(ctx, payload)
    }
}
