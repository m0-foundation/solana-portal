#![allow(unexpected_cfgs)]

mod consts;
mod instructions;
pub mod state;

use anchor_lang::prelude::*;
use consts::{
    HANDLE_ACCOUNT_METAS_DISCRIMINATOR, HANDLE_DISCRIMINATOR, ISM_DISCRIMINATOR,
    ISM_METAS_DISCRIMINATOR,
};
use instructions::*;
use m0_portal_common::Peer;

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Portal Hyperlane Adapter Program",
    project_url: "https://m0.org/",
    contacts: "email:security@m0.xyz",
    policy: "https://github.com/m0-foundation/solana-portal/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/m0-foundation/solana-portal/tree/main/programs/hyperlane-adapter",
auditors: "Halborn, Sherlock, Adevar, Guardian"
}

declare_id!("mZhPGteS36G7FhMTcRofLQU8ocBNAsGq7u8SKSHfL2X");

#[program]
pub mod hyperlane_adapter {
    use super::*;

    /// Admin Instructions

    pub fn initialize(ctx: Context<Initialize>, chain_id: u32) -> Result<()> {
        Initialize::handler(ctx, chain_id)
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

    pub fn set_peer(ctx: Context<SetPeer>, peer: Peer) -> Result<()> {
        SetPeer::handler(ctx, peer)
    }

    pub fn set_ism(ctx: Context<SetIsm>, ism: Option<Pubkey>) -> Result<()> {
        SetIsm::handler(ctx, ism)
    }

    pub fn set_igp(ctx: Context<SetIgp>) -> Result<()> {
        SetIgp::handler(ctx)
    }

    pub fn set_igp_gas_amount(ctx: Context<SetIgpGasAmount>, igp_gas_amount: u64) -> Result<()> {
        SetIgpGasAmount::handler(ctx, igp_gas_amount)
    }

    pub fn sync_extensions(ctx: Context<SyncExtensions>) -> Result<()> {
        SyncExtensions::handler(ctx)
    }

    /// Outbound Instructions

    pub fn send_message(
        ctx: Context<SendMessage>,
        m0_destination_chain_id: u32,
        message_id: [u8; 32],
        payload: Vec<u8>,
        payload_type: u8,
    ) -> Result<()> {
        SendMessage::handler(
            ctx,
            m0_destination_chain_id,
            message_id,
            payload,
            payload_type,
        )
    }

    /// Inbound Instructions

    #[instruction(discriminator = &HANDLE_DISCRIMINATOR)]
    pub fn receive_message<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        origin: u32,
        sender: [u8; 32],
        message: Vec<u8>,
    ) -> Result<()> {
        ReceiveMessage::handler(ctx, origin, sender, message)
    }

    /// Read-only Instructions

    #[instruction(discriminator = &HANDLE_ACCOUNT_METAS_DISCRIMINATOR)]
    pub fn receive_message_metas(
        ctx: Context<ReceiveMessageMetas>,
        origin: u32,
        sender: [u8; 32],
        message: Vec<u8>,
    ) -> Result<()> {
        ReceiveMessageMetas::handler(ctx, origin, sender, message)
    }

    #[instruction(discriminator = &ISM_DISCRIMINATOR)]
    pub fn get_ism(ctx: Context<GetIsm>) -> Result<()> {
        GetIsm::handler(ctx)
    }

    #[instruction(discriminator = &ISM_METAS_DISCRIMINATOR)]
    pub fn get_ism_metas(ctx: Context<GetIsmMetas>) -> Result<()> {
        GetIsmMetas::handler(ctx)
    }
}
