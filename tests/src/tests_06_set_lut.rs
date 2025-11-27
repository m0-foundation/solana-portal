use anchor_lang::AccountDeserialize;
use anyhow::Result;
use common::{
    pda,
    wormhole_adapter::{self, accounts::WormholeGlobal},
};
use std::vec;

use crate::run_surfpool_cmd;

#[test]
fn test_01_set_wormhole_lut() -> Result<()> {
    run_surfpool_cmd(vec!["run", "set_lut", "--unsupervised"])?;

    let client = crate::get_rpc_client();
    let data_wh = client.get_account_data(&pda!(&[b"global"], &wormhole_adapter::ID))?;
    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;

    assert!(global_wh.receive_lut.is_some());

    Ok(())
}
