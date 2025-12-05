use anchor_lang::prelude::*;

use crate::state::{HyperlaneGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct SetIsm<'info> {
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
    pub fn handler(ctx: Context<Self>, ism: Option<Pubkey>) -> Result<()> {
        // Set to None to use the default ISM
        ctx.accounts.hyperlane_global.ism = ism;
        Ok(())
    }
}
