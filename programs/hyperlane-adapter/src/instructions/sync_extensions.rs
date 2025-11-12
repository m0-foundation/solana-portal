use anchor_lang::prelude::*;
use common::ext_swap::accounts::SwapGlobal;

use crate::state::{
    AccountMetasData, DASH_SEED, GLOBAL_SEED, METADATA_SEED_1, METADATA_SEED_2, METADATA_SEED_3,
};

#[derive(Accounts)]
pub struct SyncExtensions<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        realloc = AccountMetasData::size(swap_global.whitelisted_extensions.len()),
        realloc::payer = payer,
        realloc::zero = false,
        seeds = [
            METADATA_SEED_1,
            DASH_SEED,
            METADATA_SEED_2,
            DASH_SEED,
            METADATA_SEED_3,
        ],
        bump = account_metas_data.bump,
    )]
    pub account_metas_data: Account<'info, AccountMetasData>,

    #[account(
        seeds = [GLOBAL_SEED],
        bump = swap_global.bump,
    )]
    pub swap_global: Account<'info, SwapGlobal>,

    pub system_program: Program<'info, System>,
}

impl SyncExtensions<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.account_metas_data.extensions = ctx
            .accounts
            .swap_global
            .whitelisted_extensions
            .iter()
            .map(|&ext| ext.into())
            .collect();

        Ok(())
    }
}
