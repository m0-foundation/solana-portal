use anchor_lang::prelude::*;
use m0_portal_common::{BridgeAdapter, BridgeError, IndexPayload, PayloadData};

use crate::{
    instructions::send_message,
    state::{PortalGlobal, GLOBAL_SEED, PORTAL_AUTHORITY_SEED},
};

#[derive(Accounts)]
pub struct SendIndex<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        constraint = !portal_global.outgoing_paused @ BridgeError::Paused,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    /// CHECK: account does not hold data
    #[account(
        seeds = [PORTAL_AUTHORITY_SEED],
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
        let payload = PayloadData::Index(IndexPayload {});

        send_message(
            ctx.accounts.bridge_adapter.to_account_info(),
            ctx.accounts.sender.to_account_info(),
            &mut ctx.accounts.portal_global,
            ctx.accounts.portal_authority.to_account_info(),
            ctx.bumps.portal_authority,
            ctx.accounts.system_program.to_account_info(),
            ctx.remaining_accounts.to_vec(),
            destination_chain_id,
            payload,
            PayloadData::INDEX_DISCRIMINANT,
        )
    }
}
