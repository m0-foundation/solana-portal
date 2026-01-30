use anchor_lang::AccountDeserialize;
use anyhow::Result;
use m0_portal_common::{
    pda,
    portal::{self, accounts::PortalGlobal},
};
use std::vec;

use crate::run_surfpool_cmd;

#[test]
fn test_01_pause_incoming() -> Result<()> {
    run_surfpool_cmd(vec![
        "run",
        "pause",
        "--unsupervised",
        "--input",
        "pause_action=pause_incoming",
    ])?;

    let client = crate::get_rpc_client();
    let data_portal = client.get_account_data(&pda!(&[b"global"], &portal::ID))?;
    let global_portal = PortalGlobal::try_deserialize(&mut data_portal.as_slice())?;

    assert!(global_portal.incoming_paused);

    Ok(())
}

#[test]
fn test_02_unpause_incoming() -> Result<()> {
    run_surfpool_cmd(vec![
        "run",
        "pause",
        "--unsupervised",
        "--input",
        "pause_action=unpause_incoming",
    ])?;

    let client = crate::get_rpc_client();
    let data_portal = client.get_account_data(&pda!(&[b"global"], &portal::ID))?;
    let global_portal = PortalGlobal::try_deserialize(&mut data_portal.as_slice())?;

    assert!(!global_portal.incoming_paused);

    Ok(())
}

#[test]
fn test_03_pause_outgoing() -> Result<()> {
    run_surfpool_cmd(vec![
        "run",
        "pause",
        "--unsupervised",
        "--input",
        "pause_action=pause_outgoing",
    ])?;

    let client = crate::get_rpc_client();
    let data_portal = client.get_account_data(&pda!(&[b"global"], &portal::ID))?;
    let global_portal = PortalGlobal::try_deserialize(&mut data_portal.as_slice())?;

    assert!(global_portal.outgoing_paused);

    Ok(())
}

#[test]
fn test_05_unpause() -> Result<()> {
    run_surfpool_cmd(vec![
        "run",
        "pause",
        "--unsupervised",
        "--input",
        "pause_action=unpause_outgoing",
    ])?;

    let client = crate::get_rpc_client();
    let data_portal = client.get_account_data(&pda!(&[b"global"], &portal::ID))?;
    let global_portal = PortalGlobal::try_deserialize(&mut data_portal.as_slice())?;

    assert!(!global_portal.outgoing_paused);

    Ok(())
}
