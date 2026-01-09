use anchor_lang::prelude::*;

use crate::state::{WormholeGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct ManagePause<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = wormhole_global.bump,
        has_one = admin,
    )]
    pub wormhole_global: Account<'info, WormholeGlobal>,
}

impl ManagePause<'_> {
    pub fn handler(
        ctx: Context<Self>,
        outgoing_paused: Option<bool>,
        incoming_paused: Option<bool>,
    ) -> Result<()> {
        if let Some(outgoing_paused) = outgoing_paused {
            ctx.accounts.wormhole_global.outgoing_paused = outgoing_paused;
        }
        if let Some(incoming_paused) = incoming_paused {
            ctx.accounts.wormhole_global.incoming_paused = incoming_paused;
        }

        msg!(
            "Worhmhole pause updated: outgoing_paused={}, incoming_paused={}",
            ctx.accounts.wormhole_global.outgoing_paused,
            ctx.accounts.wormhole_global.incoming_paused
        );

        Ok(())
    }
}
