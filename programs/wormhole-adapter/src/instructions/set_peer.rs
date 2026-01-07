use anchor_lang::prelude::*;
use common::Peer;

use crate::state::{WormholeGlobal, GLOBAL_SEED};

#[derive(Accounts)]
#[instruction(peer: Peer)]
pub struct SetPeer<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = wormhole_global.bump,
        has_one = admin,
        realloc = WormholeGlobal::size(
            wormhole_global.peers.updated_peers(peer.clone()).len()
        ),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub wormhole_global: Account<'info, WormholeGlobal>,

    pub system_program: Program<'info, System>,
}

impl SetPeer<'_> {
    pub fn handler(ctx: Context<Self>, peer: Peer) -> Result<()> {
        ctx.accounts.wormhole_global.peers = ctx
            .accounts
            .wormhole_global
            .peers
            .updated_peers(peer.clone());

        emit!(PeerSet {
            m0_chain_id: peer.m0_chain_id,
            wormhole_chain_id: peer.adapter_chain_id,
            peer: peer.address,
        });

        Ok(())
    }
}

#[event]
pub struct PeerSet {
    pub m0_chain_id: u32,
    pub wormhole_chain_id: u32,
    pub peer: [u8; 32],
}
