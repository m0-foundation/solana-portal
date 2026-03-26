use anchor_client::{Client, Cluster};
use anchor_lang::{system_program, AccountDeserialize};
use anyhow::{Ok, Result};
use hyperlane_adapter::state::{HyperlaneGlobal, GLOBAL_SEED};
use layerzero_adapter::state::LayerZeroGlobal;
use m0_portal_common::{
    consts::{
        HYPERLANE_DEFAULT_IGP_ACCOUNT, HYPERLANE_DEFAULT_IGP_PROGRAM_ID,
        HYPERLANE_DEFAULT_OVERHEAD_IGP_ACCOUNT,
    },
    hyperlane_adapter::accounts::AccountMetasData,
    pda,
    portal::{self, accounts::PortalGlobal, constants::CHAIN_PATHS_SEED},
};
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;
use std::vec;
use wormhole_adapter::state::WormholeGlobal;

use crate::{get_rpc_client, get_signer, run_surfpool_cmd, set_account};
use crate::util::constants::SOLANA_CHAIN_ID;

#[test]
fn test_01_initialize_programs() -> Result<()> {
    // reset state (which gets pulled from mainnet)
    for program_id in [portal::ID, wormhole_adapter::ID, hyperlane_adapter::ID, layerzero_adapter::ID] {
        set_account(&pda!(&[GLOBAL_SEED], &program_id), &Account::default())?;
    }

    set_account(
        &pda!(&[CHAIN_PATHS_SEED, &1u32.to_le_bytes()], &portal::ID),
        &Account::default(),
    )?;

    let logs = run_surfpool_cmd(vec!["run", "initialize", "--unsupervised"])?;
    assert!(!logs.contains("error"), "Initialization failed: {}", logs);
    Ok(())
}

#[test]
fn test_02_rerun_initailize() -> Result<()> {
    let logs = run_surfpool_cmd(vec!["run", "initialize", "--unsupervised"])?;
    assert!(logs.contains("Pre-condition failed"));
    Ok(())
}

#[test]
fn test_03_check_globals() -> Result<()> {
    let client = crate::get_rpc_client();

    let data_portal = client.get_account_data(&pda!(&[b"global"], &portal::ID))?;
    let data_wh = client.get_account_data(&pda!(&[b"global"], &wormhole_adapter::ID))?;
    let data_hyp = client.get_account_data(&pda!(&[b"global"], &hyperlane_adapter::ID))?;

    let global_portal = PortalGlobal::try_deserialize(&mut data_portal.as_slice())?;
    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;

    // Assert all fields of global_portal
    assert_eq!(global_portal.chain_id, 1399811149); // localnet chain_id
    assert_eq!(global_portal.m_index, 0);
    assert_eq!(global_portal.message_nonce, 0);
    assert_eq!(global_portal.pending_admin, None);
    assert_eq!(global_portal.unclaimed_m_balance, 0);
    assert_eq!(global_portal.padding, [0u8; 120]);
    assert!(global_portal.incoming_paused);
    assert!(global_portal.outgoing_paused);

    // Assert all fields of global_hp
    assert_eq!(global_hp.igp_program_id, HYPERLANE_DEFAULT_IGP_PROGRAM_ID);
    assert_eq!(global_hp.igp_gas_amount, 50000);
    assert_eq!(global_hp.igp_account, HYPERLANE_DEFAULT_IGP_ACCOUNT);
    assert_eq!(
        global_hp.igp_overhead_account,
        Some(HYPERLANE_DEFAULT_OVERHEAD_IGP_ACCOUNT)
    );
    assert_eq!(global_hp.ism, None);
    assert_eq!(global_hp.pending_admin, None);
    assert!(global_hp.peers.len() == 0);
    assert_eq!(global_hp.padding, [0u8; 128]);
    assert!(!global_hp.outgoing_paused);

    // Assert all fields of global_wh
    assert_eq!(global_wh.receive_lut, None);
    assert_eq!(global_wh.pending_admin, None);
    assert!(global_wh.peers.len() == 0);
    assert_eq!(global_wh.padding, [0u8; 128]);
    assert_eq!(global_wh.receive_lut, None);
    assert!(!global_wh.outgoing_paused);

    assert_eq!(global_wh.admin, global_portal.admin);
    assert_eq!(global_portal.admin, global_hp.admin);

    Ok(())
}

#[test]
fn test_04_initialize_layerzero() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(layerzero_adapter::ID)?;

    // Use system_program as a dummy endpoint since skip-validation skips register_oapp
    program
        .request()
        .accounts(layerzero_adapter::accounts::Initialize {
            admin: program.payer(),
            lz_global: pda!(&[GLOBAL_SEED], &layerzero_adapter::ID),
            endpoint_program: system_program::ID,
            system_program: system_program::ID,
        })
        .args(layerzero_adapter::instruction::Initialize {
            chain_id: SOLANA_CHAIN_ID,
        })
        .send()?;

    // Verify global state
    let rpc = get_rpc_client();
    let data = rpc.get_account_data(&pda!(&[GLOBAL_SEED], &layerzero_adapter::ID))?;
    let global = LayerZeroGlobal::try_deserialize(&mut data.as_slice())?;

    assert_eq!(global.chain_id, SOLANA_CHAIN_ID);
    assert_eq!(global.admin, signer.pubkey());
    assert!(!global.outgoing_paused);
    assert!(!global.incoming_paused);
    assert_eq!(global.peers.len(), 0);
    assert_eq!(global.pending_admin, None);

    Ok(())
}

#[test]
fn test_05_check_hyperlane_metas_pda() -> Result<()> {
    let client = crate::get_rpc_client();

    let data_account_metas = client.get_account_data(&pda!(
        &[
            b"hyperlane_message_recipient",
            b"-",
            b"handle",
            b"-",
            b"account_metas"
        ],
        &hyperlane_adapter::ID
    ))?;

    let account_metas = AccountMetasData::try_deserialize(&mut data_account_metas.as_slice())?;
    assert!(account_metas.extensions.len() > 5);

    Ok(())
}

#[test]
fn test_06_fund_hyperlane_receive_payer() -> Result<()> {
    let logs = run_surfpool_cmd(vec!["run", "fund_receive_payer", "--unsupervised"])?;
    assert!(!logs.contains("error"), "Funding failed: {}", logs);
    Ok(())
}

#[test]
fn test_07_unpause() -> Result<()> {
    run_surfpool_cmd(vec![
        "run",
        "pause",
        "--unsupervised",
        "--input",
        "pause_action=unpause_incoming",
    ])?;

    run_surfpool_cmd(vec![
        "run",
        "pause",
        "--unsupervised",
        "--input",
        "pause_action=unpause_outgoing",
    ])?;

    Ok(())
}
