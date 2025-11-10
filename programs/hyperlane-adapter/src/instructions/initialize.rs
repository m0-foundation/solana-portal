use anchor_lang::prelude::*;

use crate::state::{HyperlaneGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space =  HyperlaneGlobal::size(0),
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

    pub system_program: Program<'info, System>,
}

impl Initialize<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.hyperlane_global.set_inner(HyperlaneGlobal {
            bump: ctx.bumps.hyperlane_global,
            admin: ctx.accounts.admin.key(),
            paused: false,
            peers: Vec::new(),
        });

        Ok(())
    }
}
