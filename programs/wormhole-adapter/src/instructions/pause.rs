use anchor_lang::prelude::*;

use crate::state::{WormholeGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct Pause<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = wormhole_global.bump,
        has_one = admin,
    )]
    pub wormhole_global: Account<'info, WormholeGlobal>,
}

impl Pause<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.wormhole_global.paused = true;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Unpause<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = wormhole_global.bump,
        has_one = admin,
    )]
    pub wormhole_global: Account<'info, WormholeGlobal>,
}

impl Unpause<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.wormhole_global.paused = false;
        Ok(())
    }
}
