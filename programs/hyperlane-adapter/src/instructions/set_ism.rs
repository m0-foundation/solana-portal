use anchor_lang::prelude::*;

use crate::state::{HyperlaneGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct SetIsm<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
        has_one = admin,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,
}

impl SetIsm<'_> {
    pub fn handler(ctx: Context<Self>, ism: Pubkey) -> Result<()> {
        ctx.accounts.hyperlane_global.ism = Some(ism);
        Ok(())
    }
}
