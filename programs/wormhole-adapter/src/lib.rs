#![allow(unexpected_cfgs)]

mod consts;
mod instructions;
mod state;

use crate::state::Peer;
use anchor_lang::prelude::*;
use executor_account_resolver_svm::{InstructionGroups, Resolver, RESOLVER_EXECUTE_VAA_V1};
use instructions::*;

declare_id!("mzWh4w2CAHymGp89Z8VV2nKuCkdSFARS3fEaTBPq14b");

#[program]
pub mod wormhole_adapter {
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
