use anchor_lang::prelude::*;

use crate::state::{HyperlaneGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct Pause<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
        has_one = admin,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,
}

impl Pause<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.hyperlane_global.paused = true;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Unpause<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
        has_one = admin,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,
}

impl Unpause<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.hyperlane_global.paused = false;
        Ok(())
    }
}
