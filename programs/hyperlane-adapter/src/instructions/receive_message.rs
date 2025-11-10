use anchor_lang::prelude::*;
use common::{
    portal::{self, program::Portal},
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

    pub portal_program: Program<'info, Portal>,

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
                ctx.accounts.portal_program.to_account_info(),
                portal::cpi::accounts::ReceiveMessage {
                    sender: ctx.accounts.relayer.to_account_info(),
                    adapter_authority: ctx.accounts.hyperlane_adapter_authority.to_account_info(),
                    messenger_authority: ctx.accounts.messenger_authority.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            )
            .with_remaining_accounts(ctx.remaining_accounts.to_vec()),
            vec![],
        )
    }
}
