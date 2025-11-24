use anchor_lang::AccountDeserialize;
use anyhow::Result;
use common::{
    hyperlane_adapter::{self, accounts::HyperlaneGlobal},
    pda,
    wormhole_adapter::{self, accounts::WormholeGlobal},
};
use std::vec;

use crate::run_surfpool_cmd;

#[test]
fn test_01_set_peers() -> Result<()> {
    run_surfpool_cmd(vec!["run", "set_peers", "--unsupervised"])?;
    Ok(())
}

#[test]
fn test_02_check_globals() -> Result<()> {
    let client = crate::get_rpc_client();

    let data_wh = client.get_account_data(&pda!(&[b"global"], &wormhole_adapter::ID))?;
    let data_hyp = client.get_account_data(&pda!(&[b"global"], &hyperlane_adapter::ID))?;

    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;

    assert_eq!(global_wh.peers.len(), 4);
    assert_eq!(global_hp.peers.len(), 0);

    assert_eq!(
        global_wh.peers[0].address,
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 217, 37, 200, 75, 85, 228, 228, 74, 83, 116, 159,
            245, 242, 165, 161, 63, 99, 209, 40, 253
        ]
    );

    assert_eq!(global_wh.peers[0].chain_id, 2);
    assert_eq!(global_wh.peers[1].chain_id, 23);
    assert_eq!(global_wh.peers[2].chain_id, 24);
    assert_eq!(global_wh.peers[3].chain_id, 30);

    Ok(())
}
