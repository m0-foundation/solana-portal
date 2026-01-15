use anchor_lang::prelude::*;
use m0_portal_common::earn::{self, accounts::EarnGlobal};

use crate::state::{PortalGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space =  PortalGlobal::SIZE,
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    #[account(
        seeds = [GLOBAL_SEED],
        seeds::program = earn::ID,
        bump = earn_global.bump,
    )]
    pub earn_global: Account<'info, EarnGlobal>,

    pub system_program: Program<'info, System>,
}

impl Initialize<'_> {
    pub fn handler(
        ctx: Context<Self>,
        chain_id: u32,
        isolated_hub_chain_id: Option<u32>,
    ) -> Result<()> {
        ctx.accounts.portal_global.set_inner(PortalGlobal {
            admin: ctx.accounts.admin.key(),
            bump: ctx.bumps.portal_global,
            m_index: 0,
            m_mint: ctx.accounts.earn_global.m_mint,
            outgoing_paused: false,
            incoming_paused: false,
            chain_id,
            message_nonce: 0,
            pending_admin: None,
            isolated_hub_chain_id,
            unclaimed_m_balance: 0,
            padding: [0u8; 112],
        });

        if let Some(isolated_hub_chain_id) = isolated_hub_chain_id {
            msg!(
                "Initialized as isolated spoke connected to chain {}",
                isolated_hub_chain_id
            );
        }

        Ok(())
    }
}
