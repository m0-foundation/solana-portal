use anchor_lang::AccountDeserialize;
use anyhow::{Ok, Result};
use common::{portal::accounts::MessengerGlobal, wormhole_adapter::accounts::WormholeGlobal};
use solana_sdk::pubkey::Pubkey;
use std::{str::FromStr, vec};

use crate::run_surfpool_cmd;

#[test]
fn test_01_initialize_programs() -> Result<()> {
    run_surfpool_cmd(vec!["run", "initialize", "--unsupervised"])?;
    Ok(())
}

#[test]
fn test_02_rerun_initailize() -> Result<()> {
    let logs = run_surfpool_cmd(vec!["run", "initialize", "--unsupervised"])?;
    assert!(logs.contains("Pre-condition failed"),);
    Ok(())
}

#[test]
fn test_03_check_globals() -> Result<()> {
    let client = crate::get_rpc_client();

    let data_portal = client.get_account_data(&Pubkey::from_str(
        "54dGjbVChJseSS7zo1AWWazMtz4NXi89pQPF2HH2hM6W",
    )?)?;
    let data_wh = client.get_account_data(&Pubkey::from_str(
        "3bhczvnEexwTjdwR8b1LFDh5beYb8CLYAXUukZR8ZNdy",
    )?)?;

    let global_portal = MessengerGlobal::try_deserialize(&mut data_portal.as_slice())?;
    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;

    assert_eq!(global_portal.admin, global_wh.admin);
    assert!(!global_portal.paused);
    assert!(!global_wh.paused);

    Ok(())
}
