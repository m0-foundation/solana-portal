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
            hyperlane_global.peers.len() +
            hyperlane_global.get_peer(peer.chain_id).is_err() as usize
        ),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub hyperlane_global: Account<'info, HyperlaneGlobal>,

    pub system_program: Program<'info, System>,
}

impl SetPeer<'_> {
    pub fn handler(ctx: Context<Self>, peer: Peer) -> Result<()> {
        // Insert or overwrite peer
        match ctx
            .accounts
            .hyperlane_global
            .peers
            .iter_mut()
            .find(|p| p.chain_id == peer.chain_id)
        {
            Some(existing_peer) => *existing_peer = peer,
            None => ctx.accounts.hyperlane_global.peers.push(peer),
        }

        Ok(())
    }
}
