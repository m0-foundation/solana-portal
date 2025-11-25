use anchor_lang::prelude::*;

use crate::state::{PortalGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct Pause<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin,
    )]
    pub portal_global: Account<'info, PortalGlobal>,
}

impl Pause<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.portal_global.paused = true;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Unpause<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin,
    )]
    pub portal_global: Account<'info, PortalGlobal>,
}

impl Unpause<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.portal_global.paused = false;
        Ok(())
    }
}
