use anchor_lang::prelude::*;
use common::{
    earn::{self, accounts::EarnGlobal},
    BridgeAdapter, BridgeError, EarnerMerkleRootPayload, Payload,
};

use crate::{
    instructions::send_message,
    state::{PortalGlobal, AUTHORITY_SEED, GLOBAL_SEED},
};

#[derive(Accounts)]
pub struct SendMerkleRoot<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        constraint = !portal_global.paused @ BridgeError::Paused,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    #[account(
        seeds = [GLOBAL_SEED],
        seeds::program = earn::ID,
        bump = earn_global.bump,
    )]
    pub earn_global: Account<'info, EarnGlobal>,

    /// CHECK: account does not hold data
    #[account(
        seeds = [AUTHORITY_SEED],
        bump,
    )]
    pub portal_authority: UncheckedAccount<'info>,

    pub bridge_adapter: Interface<'info, BridgeAdapter>,

    pub system_program: Program<'info, System>,
}

impl SendMerkleRoot<'_> {
    pub fn handler<'info>(
        ctx: Context<'_, '_, '_, 'info, SendMerkleRoot<'info>>,
        destination_chain_id: u32,
    ) -> Result<()> {
        let message = Payload::EarnerMerkleRoot(EarnerMerkleRootPayload {
            index: ctx.accounts.portal_global.m_index,
            merkle_root: ctx.accounts.earn_global.earner_merkle_root,
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
