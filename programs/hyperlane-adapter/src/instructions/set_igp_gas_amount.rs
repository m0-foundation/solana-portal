use anchor_lang::prelude::*;

use crate::state::{HyperlaneGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct SetIgpGasAmount<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
        has_one = admin,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,
}

impl SetIgpGasAmount<'_> {
    pub fn handler(ctx: Context<Self>, igp_gas_amount: u64) -> Result<()> {
        ctx.accounts.hyperlane_global.igp_gas_amount = igp_gas_amount;
        Ok(())
    }
}
