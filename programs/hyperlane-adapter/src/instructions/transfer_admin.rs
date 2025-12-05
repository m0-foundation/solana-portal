use anchor_lang::prelude::*;
use common::BridgeError;

use crate::state::{HyperlaneGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct ProposeAdmin<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
        has_one = admin,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,
}

impl ProposeAdmin<'_> {
    pub fn handler(ctx: Context<Self>, new_admin: Pubkey) -> Result<()> {
        ctx.accounts.hyperlane_global.pending_admin = Some(new_admin);

        msg!(
            "Admin transfer proposed. Current admin: {}, Pending admin: {}",
            ctx.accounts.admin.key(),
            new_admin
        );

        Ok(())
    }
}

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    pub pending_admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
        constraint = hyperlane_global.pending_admin == Some(pending_admin.key()) @ BridgeError::NotAuthorized,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,
}

impl AcceptAdmin<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        let old_admin = ctx.accounts.hyperlane_global.admin;
        let new_admin = ctx.accounts.pending_admin.key();

        ctx.accounts.hyperlane_global.admin = new_admin;
        ctx.accounts.hyperlane_global.pending_admin = None;

        msg!(
            "Admin transfer completed. Old admin: {}, New admin: {}",
            old_admin,
            new_admin
        );

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CancelAdminTransfer<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
        has_one = admin,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,
}

impl CancelAdminTransfer<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.hyperlane_global.pending_admin = None;
        Ok(())
    }
}
