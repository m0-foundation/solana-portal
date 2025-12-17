use anchor_lang::AccountDeserialize;
use anyhow::Result;
use common::{
    pda,
    portal::{self, accounts::PortalGlobal},
};
use std::vec;

use crate::run_surfpool_cmd;

#[test]
fn test_01_pause() -> Result<()> {
    run_surfpool_cmd(vec![
        "run",
        "pause",
        "--unsupervised",
        "--input",
        "pause_action=pause",
    ])?;

    let client = crate::get_rpc_client();
    let data_portal = client.get_account_data(&pda!(&[b"global"], &portal::ID))?;
    let global_portal = PortalGlobal::try_deserialize(&mut data_portal.as_slice())?;

    assert!(global_portal.paused);

    Ok(())
}

#[test]
fn test_02_unpause() -> Result<()> {
    run_surfpool_cmd(vec![
        "run",
        "pause",
        "--unsupervised",
        "--input",
        "pause_action=unpause",
    ])?;

    let client = crate::get_rpc_client();
    let data_portal = client.get_account_data(&pda!(&[b"global"], &portal::ID))?;
    let global_portal = PortalGlobal::try_deserialize(&mut data_portal.as_slice())?;

    assert!(!global_portal.paused);

    Ok(())
}
