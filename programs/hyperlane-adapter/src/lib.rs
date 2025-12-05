#![allow(unexpected_cfgs)]

mod consts;
mod instructions;
mod state;

use crate::state::Peer;
use anchor_lang::prelude::*;
use consts::{
    HANDLE_ACCOUNT_METAS_DISCRIMINATOR, HANDLE_DISCRIMINATOR, ISM_DISCRIMINATOR,
    ISM_METAS_DISCRIMINATOR,
};
use instructions::*;

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Portal Hyperlane Adapter Program",
    project_url: "https://m0.org/",
    contacts: "email:security@m0.xyz",
    policy: "https://github.com/m0-foundation/solana-portal/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/m0-foundation/solana-portal/tree/main/programs/hyperlane-adapter",
    auditors: ""
}

declare_id!("mZhPGteS36G7FhMTcRofLQU8ocBNAsGq7u8SKSHfL2X");

#[program]
pub mod hyperlane_adapter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Initialize::handler(ctx)
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

    pub fn set_peer(ctx: Context<SetPeer>, peer: Peer) -> Result<()> {
        SetPeer::handler(ctx, peer)
    }

    pub fn sync_extensions(ctx: Context<SyncExtensions>) -> Result<()> {
        SyncExtensions::handler(ctx)
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

    pub fn send_message(
        ctx: Context<SendMessage>,
        message: Vec<u8>,
        destination_chain_id: u32,
    ) -> Result<()> {
        SendMessage::handler(ctx, message, destination_chain_id)
    }

    #[instruction(discriminator = &HANDLE_DISCRIMINATOR)]
    pub fn receive_message<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        origin: u32,
        sender: [u8; 32],
        message: Vec<u8>,
    ) -> Result<()> {
        ReceiveMessage::handler(ctx, origin, sender, message)
    }

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
