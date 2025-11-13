use anchor_lang::AccountDeserialize;
use anyhow::{Ok, Result};
use common::{
    hyperlane_adapter::{self, accounts::HyperlaneGlobal},
    pda,
    portal::{self, accounts::PortalGlobal},
    wormhole_adapter::{self, accounts::WormholeGlobal},
};
use solana_sdk::pubkey::Pubkey;
use std::vec;

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

    let data_portal = client.get_account_data(&pda!(&[b"global"], &portal::ID))?;
    let data_wh = client.get_account_data(&pda!(&[b"global"], &wormhole_adapter::ID))?;
    let data_hyp = client.get_account_data(&pda!(&[b"global"], &hyperlane_adapter::ID))?;

    let global_portal = PortalGlobal::try_deserialize(&mut data_portal.as_slice())?;
    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;

    assert_eq!(global_portal.admin, global_wh.admin);
    assert_eq!(global_portal.admin, global_hp.admin);
    assert!(!global_portal.paused);
    assert!(!global_wh.paused);
    assert!(!global_hp.paused);

    Ok(())
}
