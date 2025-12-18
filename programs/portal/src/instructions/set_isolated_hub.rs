use anchor_lang::prelude::*;

use crate::state::{PortalGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct SetIsolatedHub<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin,
    )]
    pub portal_global: Account<'info, PortalGlobal>,
}

impl SetIsolatedHub<'_> {
    pub fn handler(ctx: Context<Self>, chain_id: Option<u32>) -> Result<()> {
        msg!(
            "Setting isolated hub chain ID: {:?} -> {:?}",
            ctx.accounts.portal_global.isolated_hub_chain_id,
            chain_id
        );
        ctx.accounts.portal_global.isolated_hub_chain_id = chain_id;
        Ok(())
    }
}
