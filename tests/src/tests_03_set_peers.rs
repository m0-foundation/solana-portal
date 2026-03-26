use anchor_client::{Client, Cluster};
use anchor_lang::{system_program, AccountDeserialize};
use anyhow::Result;
use hyperlane_adapter::state::HyperlaneGlobal;
use layerzero_adapter::state::LayerZeroGlobal;
use m0_portal_common::{pda, Peer};
use portal::state::{ChainBridgePaths, CHAIN_PATHS_SEED, GLOBAL_SEED};
use std::vec;
use wormhole_adapter::{accounts, instruction, state::WormholeGlobal};

use crate::{
    get_rpc_client, get_signer, run_surfpool_cmd,
    util::constants::{ETHEREUM_LAYERZERO_ADAPTER, ETHEREUM_LZ_EID, ETHEREUM_WORMHOLE_ADAPTER},
};

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

    assert_eq!(global_wh.peers.0[0].address, ETHEREUM_WORMHOLE_ADAPTER);

    assert_eq!(global_wh.peers.0[0].m0_chain_id, 1);
    assert_eq!(global_hp.peers.0[0].m0_chain_id, 1);

    Ok(())
}

#[test]
fn test_03_set_layerzero_peer() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let rpc_client = get_rpc_client();
    let program = client.program(layerzero_adapter::ID)?;

    program
        .request()
        .accounts(layerzero_adapter::accounts::SetPeer {
            admin: program.payer(),
            lz_global: pda!(&[GLOBAL_SEED], &layerzero_adapter::ID),
            system_program: system_program::ID,
        })
        .args(layerzero_adapter::instruction::SetPeer {
            peer: Peer {
                m0_chain_id: 1,
                address: ETHEREUM_LAYERZERO_ADAPTER,
                adapter_chain_id: ETHEREUM_LZ_EID,
            },
        })
        .send()?;

    let data_lz = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &layerzero_adapter::ID))?;
    let global_lz = LayerZeroGlobal::try_deserialize(&mut data_lz.as_slice())?;

    assert_eq!(global_lz.peers.len(), 1);
    assert_eq!(global_lz.peers.0[0].m0_chain_id, 1);
    assert_eq!(global_lz.peers.0[0].adapter_chain_id, ETHEREUM_LZ_EID);
    assert_eq!(global_lz.peers.0[0].address, ETHEREUM_LAYERZERO_ADAPTER);

    Ok(()
    )
}

#[test]
fn test_04_update_peer() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let rpc_client = get_rpc_client();

    let program = client.program(wormhole_adapter::ID)?;

    // Update wormhole chain id
    program
        .request()
        .accounts(accounts::SetPeer {
            admin: program.payer(),
            wormhole_global: pda!(&[GLOBAL_SEED], &wormhole_adapter::ID),
            system_program: system_program::ID,
        })
        .args(instruction::SetPeer {
            peer: Peer {
                m0_chain_id: 8453,
                address: [1; 32],
                adapter_chain_id: 420,
            },
        })
        .send()?;

    let data_wh = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &wormhole_adapter::ID))?;
    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;
    assert!(
        global_wh
            .peers
            .0
            .iter()
            .find(|p| p.m0_chain_id == 8453)
            .expect("did not find updated peer")
            .adapter_chain_id
            == 420
    );

    Ok(())
}

#[test]
fn test_05_remove_peer() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let rpc_client = get_rpc_client();

    let program = client.program(wormhole_adapter::ID)?;

    let data_wh = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &wormhole_adapter::ID))?;
    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;
    assert!(global_wh.peers.0.iter().any(|p| p.m0_chain_id == 8453));

    // Remove Base
    program
        .request()
        .accounts(accounts::SetPeer {
            admin: program.payer(),
            wormhole_global: pda!(&[GLOBAL_SEED], &wormhole_adapter::ID),
            system_program: system_program::ID,
        })
        .args(instruction::SetPeer {
            peer: Peer {
                m0_chain_id: 8453,
                address: [0; 32],
                adapter_chain_id: 420,
            },
        })
        .send()?;

    let data_wh = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &wormhole_adapter::ID))?;
    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;
    assert!(global_wh.peers.0.iter().all(|p| p.m0_chain_id != 8453));

    Ok(())
}

#[test]
fn test_06_bridge_path_config() -> Result<()> {
    let client = get_rpc_client();

    let data =
        client.get_account_data(&pda!(&[CHAIN_PATHS_SEED, &1u32.to_le_bytes()], &portal::ID))?;
    let paths = ChainBridgePaths::try_deserialize(&mut data.as_slice())?;

    assert_eq!(paths.destination_chain_id, 1u32);
    assert_eq!(paths.paths.len(), 0);

    Ok(())
}
