use anchor_lang::prelude::*;
use common::earn::{self, accounts::EarnGlobal};

use crate::{
    consts::{
        DEFAULT_HANDLE_GAS_AMOUNT, DEFAULT_IGP_ACCOUNT, DEFAULT_IGP_PROGRAM_ID,
        DEFAULT_OVERHEAD_IGP_ACCOUNT,
    },
    state::{
        AccountMetasData, HyperlaneGlobal, DASH_SEED, GLOBAL_SEED, METADATA_SEED_1,
        METADATA_SEED_2, METADATA_SEED_3,
    },
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space =  HyperlaneGlobal::size(0),
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

    #[account(
        init,
        payer = admin,
        space =  AccountMetasData::size(0),
        seeds = [
            METADATA_SEED_1,
            DASH_SEED,
            METADATA_SEED_2,
            DASH_SEED,
            METADATA_SEED_3,
        ],
        bump
    )]
    pub account_metas_data: Account<'info, AccountMetasData>,

    #[account(
        seeds = [GLOBAL_SEED],
        seeds::program = earn::ID,
        bump = earn_global.bump,
    )]
    pub earn_global: Account<'info, EarnGlobal>,

    pub system_program: Program<'info, System>,
}

impl Initialize<'_> {
    pub fn handler(ctx: Context<Self>) -> Result<()> {
        ctx.accounts.hyperlane_global.set_inner(HyperlaneGlobal {
            bump: ctx.bumps.hyperlane_global,
            nonce: 0,
            admin: ctx.accounts.admin.key(),
            ism: None,
            paused: false,
            peers: Vec::new(),
            pending_admin: None,
            igp_program_id: DEFAULT_IGP_PROGRAM_ID,
            igp_account: DEFAULT_IGP_ACCOUNT,
            igp_overhead_account: Some(DEFAULT_OVERHEAD_IGP_ACCOUNT),
            igp_gas_amount: DEFAULT_HANDLE_GAS_AMOUNT,
        });

        ctx.accounts.account_metas_data.set_inner(AccountMetasData {
            bump: ctx.bumps.account_metas_data,
            m_mint: ctx.accounts.earn_global.m_mint,
            extensions: Vec::new(),
        });

        Ok(())
    }
}
