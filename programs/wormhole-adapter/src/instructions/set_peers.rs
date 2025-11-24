use anchor_lang::prelude::*;

use crate::state::{Peer, WormholeGlobal, GLOBAL_SEED};

#[derive(Accounts)]
#[instruction(peers: Vec<Peer>)]
pub struct SetPeers<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump,
        has_one = admin,
        realloc = WormholeGlobal::size(
            wormhole_global.extended_peers(peers.clone()).len()
        ),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub wormhole_global: Account<'info, WormholeGlobal>,

    pub system_program: Program<'info, System>,
}

impl SetPeers<'_> {
    pub fn handler(ctx: Context<Self>, peers: Vec<Peer>) -> Result<()> {
        ctx.accounts.wormhole_global.peers = ctx.accounts.wormhole_global.extended_peers(peers);
        Ok(())
    }
}
