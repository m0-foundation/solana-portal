use anchor_lang::prelude::*;

use crate::state::{HyperlaneGlobal, Peer, GLOBAL_SEED};

#[derive(Accounts)]
#[instruction(peer: Peer)]
pub struct SetPeer<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = hyperlane_global.bump,
        has_one = admin,
        realloc = HyperlaneGlobal::size(
            hyperlane_global.updated_peers(peer.clone()).len()
        ),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

    pub system_program: Program<'info, System>,
}

impl SetPeer<'_> {
    pub fn handler(ctx: Context<Self>, peer: Peer) -> Result<()> {
        ctx.accounts.hyperlane_global.peers = ctx.accounts.hyperlane_global.updated_peers(peer);

        Ok(())
    }
}
