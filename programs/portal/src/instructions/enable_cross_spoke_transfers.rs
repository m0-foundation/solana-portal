use anchor_lang::prelude::*;

use crate::state::{PortalGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct EnableCrossSpokeTransfers<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin,
        constraint = portal_global.isolated_hub_chain_id.is_some()
    )]
    pub portal_global: Account<'info, PortalGlobal>,
}

impl EnableCrossSpokeTransfers<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.portal_global.isolated_hub_chain_id = None;
        Ok(())
    }
}
