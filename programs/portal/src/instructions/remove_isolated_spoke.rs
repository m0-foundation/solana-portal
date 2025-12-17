use anchor_lang::prelude::*;
use common::BridgeError;

use crate::state::{PortalGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct RemoveIsolatedSpoke<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = portal_global.bump,
        has_one = admin,
        realloc = PortalGlobal::size(portal_global.isolated_spokes.len() - 1),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub portal_global: Account<'info, PortalGlobal>,

    pub system_program: Program<'info, System>,
}

impl RemoveIsolatedSpoke<'_> {
    fn validate(&self, chain_id: u32) -> Result<()> {
        require!(
            self.portal_global
                .isolated_spokes
                .iter()
                .any(|spoke| spoke.chain_id == chain_id),
            BridgeError::ChainNotIsolated
        );

        Ok(())
    }

    #[access_control(ctx.accounts.validate(chain_id))]
    pub fn handler(ctx: Context<Self>, chain_id: u32) -> Result<()> {
        // Remove the isolated spoke with the given chain_id
        ctx.accounts
            .portal_global
            .isolated_spokes
            .retain(|spoke| spoke.chain_id != chain_id);

        Ok(())
    }
}
