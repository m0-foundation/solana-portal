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

    pub fn initialize(
        ctx: Context<Initialize>,
        chain_id: u32,
        isolated_hub_chain_id: Option<u32>,
    ) -> Result<()> {
        Initialize::handler(ctx, chain_id, isolated_hub_chain_id)
    }

    pub fn pause_outgoing(ctx: Context<ManagePause>) -> Result<()> {
        ManagePause::handler(ctx, Some(true), None)
    }

    pub fn unpause_outgoing(ctx: Context<ManagePause>) -> Result<()> {
        ManagePause::handler(ctx, Some(false), None)
    }

    pub fn pause_incoming(ctx: Context<ManagePause>) -> Result<()> {
        ManagePause::handler(ctx, None, Some(true))
    }

    pub fn unpause_incoming(ctx: Context<ManagePause>) -> Result<()> {
        ManagePause::handler(ctx, None, Some(false))
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

    pub fn enable_cross_spoke_transfers(ctx: Context<EnableCrossSpokeTransfers>) -> Result<()> {
        EnableCrossSpokeTransfers::handler(ctx)
    }

    pub fn claim_m_balance(ctx: Context<ClaimMBalance>, amount: Option<u64>) -> Result<()> {
        ClaimMBalance::handler(ctx, amount)
    }

    /// Bridge Path Configuration Instructions

    pub fn initialize_chain_paths(
        ctx: Context<InitializeChainPaths>,
        destination_chain_id: u32,
    ) -> Result<()> {
        InitializeChainPaths::handler(ctx, destination_chain_id)
    }

    pub fn add_bridge_path(
        ctx: Context<AddBridgePath>,
        destination_chain_id: u32,
        source_mint: Pubkey,
        destination_token: [u8; 32],
    ) -> Result<()> {
        AddBridgePath::handler(ctx, destination_chain_id, source_mint, destination_token)
    }

    pub fn remove_bridge_path(
        ctx: Context<RemoveBridgePath>,
        destination_chain_id: u32,
        source_mint: Pubkey,
        destination_token: [u8; 32],
    ) -> Result<()> {
        RemoveBridgePath::handler(ctx, destination_chain_id, source_mint, destination_token)
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
        ctx: Context<'_, '_, '_, 'info, SendToken<'info>>,
        amount: u64,
        destination_token: [u8; 32],
        destination_chain_id: u32,
        recipient: [u8; 32],
    ) -> Result<()> {
        SendToken::handler(
            ctx,
            amount,
            destination_token,
            destination_chain_id,
            recipient,
        )
    }

    pub fn send_fill_report<'info>(
        ctx: Context<'_, '_, '_, 'info, SendReport<'info>>,
        order_id: [u8; 32],
        token_in: [u8; 32],
        amount_in_to_release: u128,
        amount_out_filled: u128,
        origin_recipient: [u8; 32],
        origin_chain_id: u32,
    ) -> Result<()> {
        SendReport::send_fill_report_handler(
            ctx,
            order_id,
            token_in,
            amount_in_to_release,
            amount_out_filled,
            origin_recipient,
            origin_chain_id,
        )
    }

    pub fn send_cancel_report<'info>(
        ctx: Context<'_, '_, '_, 'info, SendReport<'info>>,
        order_id: [u8; 32],
        order_sender: [u8; 32],
        token_in: [u8; 32],
        amount_in_to_refund: u128,
        origin_chain_id: u32,
    ) -> Result<()> {
        SendReport::send_cancel_report_handler(
            ctx,
            order_id,
            order_sender,
            token_in,
            amount_in_to_refund,
            origin_chain_id,
        )
    }

    /// Inbound Instructions

    pub fn receive_message<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        message_id: [u8; 32],
        source_chain_id: u32,
        payload: Vec<u8>,
    ) -> Result<()> {
        ReceiveMessage::handler(ctx, message_id, source_chain_id, payload)
    }
}
