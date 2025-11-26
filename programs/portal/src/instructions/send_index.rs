use anchor_lang::prelude::*;
use common::{BridgeAdapter, BridgeError, IndexPayload, Payload};

use crate::{
    instructions::send_message,
    state::{PortalGlobal, AUTHORITY_SEED, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct SendIndex<'info> {
    pub sender: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        constraint = !portal_global.paused @ BridgeError::Paused,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    /// CHECK: account does not hold data
    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    pub portal_authority: UncheckedAccount<'info>,

    pub bridge_adapter: Interface<'info, BridgeAdapter>,

    pub system_program: Program<'info, System>,
}

impl SendIndex<'_> {
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, SendIndex<'info>>,
        destination_chain_id: u32,
    ) -> Result<()> {
        let message = Payload::Index(IndexPayload {
            index: ctx.accounts.portal_global.m_index,
            message_id: ctx.accounts.portal_global.generate_message_id(),
        });

        send_message(
            ctx.accounts.bridge_adapter.to_account_info(),
            ctx.accounts.sender.to_account_info(),
            ctx.accounts.portal_authority.to_account_info(),
            ctx.bumps.portal_authority,
            ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts.to_vec(),
            message.encode(),
            destination_chain_id,
        )
    }
}
