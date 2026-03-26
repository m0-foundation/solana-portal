use anchor_lang::prelude::*;
use m0_portal_common::Peer;

use crate::state::{LayerZeroGlobal, GLOBAL_SEED};

#[derive(Accounts)]
#[instruction(peer: Peer)]
pub struct SetPeer<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED],
        bump = lz_global.bump,
        has_one = admin,
        realloc = LayerZeroGlobal::size(
            lz_global.peers.updated_peers(peer.clone()).len()
        ),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub lz_global: Account<'info, LayerZeroGlobal>,

    pub system_program: Program<'info, System>,
}

impl SetPeer<'_> {
    pub fn handler(ctx: Context<Self>, peer: Peer) -> Result<()> {
        ctx.accounts.lz_global.peers = ctx
            .accounts
            .lz_global
            .peers
            .updated_peers(peer.clone());

        emit!(PeerSet {
            m0_chain_id: peer.m0_chain_id,
            lz_eid: peer.adapter_chain_id,
            peer: peer.address,
        });

        Ok(())
    }
}

#[event]
pub struct PeerSet {
    pub m0_chain_id: u32,
    pub lz_eid: u32,
    pub peer: [u8; 32],
}
