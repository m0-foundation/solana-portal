use anchor_lang::prelude::*;
use common::portal;

use crate::state::{HyperlaneGlobal, GLOBAL_SEED};

#[derive(Accounts)]
pub struct SendMessage<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    #[account(
        constraint = !hyperlane_global.paused,
        seeds = [GLOBAL_SEED],
        bump,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

    #[account(
        seeds = [b"authority"], 
        seeds::program = portal::ID,
        bump
    )]
    /// Only relay messages coming from the Portal
    messenger_authority: Signer<'info>,
}

impl SendMessage<'_> {
    pub fn handler(ctx: Context<Self>, message: Vec<u8>) -> Result<()> {
        Ok(())
    }
}
