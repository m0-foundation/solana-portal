use anchor_lang::prelude::*;

use crate::state::{PortalGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct ManagePause<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin,
    )]
    pub portal_global: Account<'info, PortalGlobal>,
}

impl ManagePause<'_> {
    pub fn handler(
        ctx: Context<Self>,
        outgoing_paused: Option<bool>,
        incoming_paused: Option<bool>,
    ) -> Result<()> {
        if let Some(outgoing_paused) = outgoing_paused {
            ctx.accounts.portal_global.outgoing_paused = outgoing_paused;
        }
        if let Some(incoming_paused) = incoming_paused {
            ctx.accounts.portal_global.incoming_paused = incoming_paused;
        }

        msg!(
            "Portal pause updated: outgoing_paused={}, incoming_paused={}",
            ctx.accounts.portal_global.outgoing_paused,
            ctx.accounts.portal_global.incoming_paused
        );

        Ok(())
    }
}
