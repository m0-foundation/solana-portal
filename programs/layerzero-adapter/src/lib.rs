#![allow(unexpected_cfgs)]

pub mod consts;
mod instructions;
pub mod state;

use anchor_lang::prelude::*;
use instructions::*;
use m0_portal_common::Peer;

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Portal LayerZero Adapter Program",
    project_url: "https://m0.org/",
    contacts: "email:security@m0.xyz",
    policy: "https://github.com/m0-foundation/solana-portal/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/m0-foundation/solana-portal/tree/main/programs/layerzero-adapter",
    auditors: ""
}

declare_id!("MzLzScr2JSzmxfDfg38ZPsw7RhRUGwkJtr2whLo7uru");

#[program]
pub mod layerzero_adapter {
    use super::*;

    /// Admin Instructions

    pub fn initialize<'info>(
        ctx: Context<'_, '_, '_, 'info, Initialize<'info>>,
        chain_id: u32,
    ) -> Result<()> {
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

    pub fn set_delegate<'info>(
        ctx: Context<'_, '_, '_, 'info, SetDelegate<'info>>,
        delegate: Pubkey,
    ) -> Result<()> {
        SetDelegate::handler(ctx, delegate)
    }

    /// Outbound Instructions

    pub fn send_message<'info>(
        ctx: Context<'_, '_, '_, 'info, SendMessage<'info>>,
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

    /// Fee Estimation

    pub fn quote<'info>(
        ctx: Context<'_, '_, '_, 'info, Quote<'info>>,
        dst_eid: u32,
        receiver: [u8; 32],
        message: Vec<u8>,
        options: Vec<u8>,
        pay_in_lz_token: bool,
    ) -> Result<()> {
        Quote::handler(ctx, dst_eid, receiver, message, options, pay_in_lz_token)
    }

    /// Inbound Instructions

    pub fn lz_receive<'info>(
        ctx: Context<'_, '_, '_, 'info, LzReceive<'info>>,
        params: state::LzReceiveParams,
    ) -> Result<()> {
        LzReceive::handler(ctx, params)
    }

    /// Read-only Instructions

    pub fn lz_receive_types<'info>(
        ctx: Context<'_, '_, '_, 'info, LzReceiveTypes<'info>>,
        params: state::LzReceiveParams,
    ) -> Result<()> {
        LzReceiveTypes::handler(ctx, params)
    }
}
