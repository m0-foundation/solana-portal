use anchor_lang::prelude::*;

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
            outgoing_paused: false,
            incoming_paused: false,
            chain_id,
            message_nonce: 0,
            pending_admin: None,
            isolated_hub_chain_id,
            padding: [0u8; 128],
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
