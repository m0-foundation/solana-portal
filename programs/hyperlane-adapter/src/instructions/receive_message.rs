use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use common::{
    earn::{self, accounts::EarnGlobal, program::Earn},
    portal,
    wormhole_verify_vaa_shim::program::WormholeVerifyVaaShim,
    AUTHORITY_SEED,
};

use crate::state::{HyperlaneGlobal, GLOBAL_SEED};

#[derive(Accounts)]
#[instruction(_guardian_set_index: u32)]
pub struct ReceiveMessage<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,

    #[account(
        constraint = !hyperlane_global.paused,
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

    #[account(
        seeds = [AUTHORITY_SEED],
        bump
    )]
    /// CHECK: Account does not hold data
    pub hyperlane_adapter_authority: AccountInfo<'info>,

    #[account(
        seeds = [AUTHORITY_SEED],
        seeds::program = portal::ID,
        bump
    )]
    /// CHECK: Account does not hold data
    pub messenger_authority: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        seeds::program = earn::ID,
        bump = m_global.bump,
        has_one = m_mint,
    )]
    pub m_global: Account<'info, EarnGlobal>,

    #[account(mut)]
    pub m_mint: InterfaceAccount<'info, Mint>,

    pub wormhole_verify_vaa_shim: Program<'info, WormholeVerifyVaaShim>,

    pub earn_program: Program<'info, Earn>,

    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

impl ReceiveMessage<'_> {
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    #[access_control(ctx.accounts.validate())]
    pub fn handler<'info>(ctx: Context<'_, '_, '_, 'info, ReceiveMessage<'info>>) -> Result<()> {
        portal::cpi::receive_message(
            CpiContext::new(
                ctx.accounts.wormhole_verify_vaa_shim.to_account_info(),
                portal::cpi::accounts::ReceiveMessage {
                    relayer: ctx.accounts.relayer.to_account_info(),
                    adapter_authority: ctx.accounts.hyperlane_adapter_authority.to_account_info(),
                    messenger_authority: ctx.accounts.messenger_authority.to_account_info(),
                    m_global: ctx.accounts.m_global.to_account_info(),
                    m_mint: ctx.accounts.m_mint.to_account_info(),
                    earn_program: ctx.accounts.earn_program.to_account_info(),
                    m_token_program: ctx.accounts.token_program.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            )
            .with_remaining_accounts(ctx.remaining_accounts.to_vec()),
            vec![],
        )
    }
}
