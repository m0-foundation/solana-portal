use anchor_lang::prelude::*;

use crate::state::{LayerZeroGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct ManagePause<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = lz_global.bump,
        has_one = admin,
    )]
    pub lz_global: Account<'info, LayerZeroGlobal>,
}

impl ManagePause<'_> {
    pub fn handler(
        ctx: Context<Self>,
        outgoing_paused: Option<bool>,
        incoming_paused: Option<bool>,
    ) -> Result<()> {
        if let Some(outgoing_paused) = outgoing_paused {
            ctx.accounts.lz_global.outgoing_paused = outgoing_paused;
        }

        if let Some(incoming_paused) = incoming_paused {
            ctx.accounts.lz_global.incoming_paused = incoming_paused;
        }

        msg!(
            "LayerZero pause updated: outgoing_paused={}, incoming_paused={}",
            ctx.accounts.lz_global.outgoing_paused,
            ctx.accounts.lz_global.incoming_paused
        );

        Ok(())
    }
}
