use anchor_lang::prelude::*;

use crate::state::{Peer, WormholeGlobal, GLOBAL_SEED};

#[derive(Accounts)]
#[instruction(peer: Peer)]
pub struct SetPeer<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump,
        has_one = admin,
        realloc = WormholeGlobal::size(
            wormhole_global.peers.len() +
            wormhole_global.get_peer(peer.chain_id).is_err() as usize
        ),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub wormhole_global: Account<'info, WormholeGlobal>,

    pub system_program: Program<'info, System>,
}

impl SetPeer<'_> {
    pub fn handler(ctx: Context<Self>, peer: Peer) -> Result<()> {
        // Insert or overwrite peer
        match ctx
            .accounts
            .wormhole_global
            .peers
            .iter_mut()
            .find(|p| p.chain_id == peer.chain_id)
        {
            Some(existing_peer) => *existing_peer = peer,
            None => ctx.accounts.wormhole_global.peers.push(peer),
        }

        Ok(())
    }
}
