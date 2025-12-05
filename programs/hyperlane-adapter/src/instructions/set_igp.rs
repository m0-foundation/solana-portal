use anchor_lang::prelude::*;

use crate::state::{HyperlaneGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct SetIgp<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
        has_one = admin,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

    /// CHECK: admin can set any program
    #[account(executable)]
    pub igp_program_id: AccountInfo<'info>,

    /// CHECK: admin can set any account owned by the IGP program
    #[account(owner = igp_program_id.key())]
    pub igp_account: AccountInfo<'info>,

    /// CHECK: admin can set any account owned by the IGP program
    #[account(owner = igp_program_id.key())]
    pub igp_overhead_account: Option<AccountInfo<'info>>,
}

impl SetIgp<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.hyperlane_global.igp_program_id = ctx.accounts.igp_program_id.key();
        ctx.accounts.hyperlane_global.igp_account = ctx.accounts.igp_account.key();
        ctx.accounts.hyperlane_global.igp_overhead_account = ctx
            .accounts
            .igp_overhead_account
            .as_ref()
            .map(|acc| acc.key());

        Ok(())
    }
}
