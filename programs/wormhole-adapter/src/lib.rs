#![allow(unexpected_cfgs)]

mod consts;
mod instructions;
mod state;

use crate::state::Peer;
use anchor_lang::prelude::*;
use executor_account_resolver_svm::{InstructionGroups, Resolver, RESOLVER_EXECUTE_VAA_V1};
use instructions::*;

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Portal Wormhole Adapter Program",
    project_url: "https://m0.org/",
    contacts: "email:security@m0.xyz",
    policy: "https://github.com/m0-foundation/solana-portal/blob/main/SECURITY.md",
    preferred_languages: "en",
    source_code: "https://github.com/m0-foundation/solana-portal/tree/main/programs/wormhole-adapter",
    auditors: ""
}

declare_id!("mzp1q2j5Hr1QuLC3KFBCAUz5aUckT6qyuZKZ3WJnMmY");

#[program]
pub mod wormhole_adapter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, recent_slot: u64) -> Result<()> {
        Initialize::handler(ctx, recent_slot)
    }

    pub fn set_lut(
        ctx: Context<SetLookupTable>,
        recent_slot: u64,
        additional_accounts: Vec<Pubkey>,
    ) -> Result<()> {
        SetLookupTable::handler(ctx, recent_slot, additional_accounts)
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

    pub fn send_message(
        ctx: Context<SendMessage>,
        message: Vec<u8>,
        destination_chain_id: u32,
    ) -> Result<()> {
        SendMessage::handler(ctx, message, destination_chain_id)
    }

    pub fn receive_message<'info>(
        ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>,
        guardian_set_index: u32,
        vaa_body: Vec<u8>,
    ) -> Result<()> {
        ReceiveMessage::handler(ctx, guardian_set_index, vaa_body)
    }

    #[instruction(discriminator = &RESOLVER_EXECUTE_VAA_V1)]
    pub fn resolve_execute<'info>(
        ctx: Context<'_, '_, 'info, 'info, ResolveExecuteVaa>,
        vaa_body: Vec<u8>,
    ) -> Result<Resolver<InstructionGroups>> {
        ResolveExecuteVaa::handler(ctx, vaa_body)
    }
}
