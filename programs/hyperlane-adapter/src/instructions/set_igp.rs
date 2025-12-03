use anchor_lang::prelude::*;

use crate::state::{HyperlaneGlobal, IgpType, GLOBAL_SEED};

#[derive(Accounts)]
pub struct SetIgp<'info> {
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

impl SetIgp<'_> {
    pub fn handler(ctx: Context<Self>, igp: Pubkey, igp_type: IgpType) -> Result<()> {
        ctx.accounts.hyperlane_global.igp = igp;
        ctx.accounts.hyperlane_global.igp_type = igp_type;
        Ok(())
    }
}
