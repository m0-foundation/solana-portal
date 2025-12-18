use anchor_client::{Client, Cluster};
use anchor_lang::{system_program, AccountDeserialize};
use anyhow::Result;
use common::{
    hyperlane_adapter::{self, accounts::HyperlaneGlobal},
    pda,
};
use portal::state::GLOBAL_SEED;
use std::vec;
use wormhole_adapter::{
    accounts, instruction,
    state::{Peer, WormholeGlobal},
};

use crate::{get_rpc_client, get_signer, run_surfpool_cmd};

#[test]
fn test_01_set_peers() -> Result<()> {
    let logs = run_surfpool_cmd(vec!["run", "set_peers", "--unsupervised"])?;
    assert!(!logs.contains("error"), "Set peers failed: {}", logs);
    Ok(())
}

#[test]
fn test_02_check_globals() -> Result<()> {
    let client = get_rpc_client();

    let data_wh = client.get_account_data(&pda!(&[GLOBAL_SEED], &wormhole_adapter::ID))?;
    let data_hyp = client.get_account_data(&pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID))?;

    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;

    assert!(global_wh.peers.len() > 0);
    assert!(global_hp.peers.len() > 0);

    assert_eq!(
        global_wh.peers[0].address,
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 99, 25, 106, 9, 21, 117, 173, 249, 158, 35, 6,
            229, 233, 14, 11, 229, 21, 72, 65
        ]
    );

    assert_eq!(global_wh.peers[0].m0_chain_id, 1);
    assert_eq!(global_hp.peers[0].m0_chain_id, 1);
    assert_eq!(global_wh.peers[1].m0_chain_id, 42161);
    assert_eq!(global_wh.peers[2].m0_chain_id, 10);
    assert_eq!(global_wh.peers[3].m0_chain_id, 8453);

    Ok(())
}

#[test]
fn test_03_remove_peer() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let rpc_client = get_rpc_client();

    let program = client.program(wormhole_adapter::ID)?;

    // Remove Optimism
    program
        .request()
        .accounts(accounts::SetPeer {
            admin: program.payer(),
            wormhole_global: pda!(&[GLOBAL_SEED], &wormhole_adapter::ID),
            system_program: system_program::ID,
        })
        .args(instruction::SetPeer {
            peer: Peer {
                m0_chain_id: 10,
                address: [0; 32],
                wormhole_chain_id: 24,
            },
        })
        .send()?;

    let data_wh = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &wormhole_adapter::ID))?;
    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;
    assert!(global_wh.peers.iter().all(|p| p.m0_chain_id != 10));

    Ok(())
}
