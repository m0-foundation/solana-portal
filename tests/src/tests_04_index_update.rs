use anchor_client::{Client, Cluster};
use anchor_lang::system_program;
use anyhow::Result;
use common::{
    pda,
    wormhole_adapter::{self},
    WormholeRemainingAccounts,
};
use portal::{accounts, instruction};

use crate::get_signer;

#[test]
fn test_01_index_update_wormhole() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());

    let program = client.program(portal::ID)?;

    program
        .request()
        .accounts(accounts::SendIndex {
            sender: program.payer(),
            system_program: system_program::ID,
            portal_global: pda!(&[b"global"], &portal::ID),
            messenger_authority: pda!(&[b"authority"], &portal::ID),
            bridge_adapter: wormhole_adapter::ID,
        })
        .args(instruction::SendIndex {
            destination_chain_id: 1,
        })
        .accounts(WormholeRemainingAccounts::account_metas())
        .send()?;

    Ok(())
}

#[test]
fn test_02_index_update_hyperlane() -> Result<()> {
    Ok(())
}
