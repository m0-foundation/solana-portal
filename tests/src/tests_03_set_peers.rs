use anchor_lang::AccountDeserialize;
use anyhow::{Ok, Result};
use common::{
    hyperlane_adapter::{self, accounts::HyperlaneGlobal},
    pda,
    wormhole_adapter::{self, accounts::WormholeGlobal},
};
use solana_sdk::pubkey::Pubkey;
use std::vec;

use crate::run_surfpool_cmd;

#[test]
fn test_01_set_peers() -> Result<()> {
    run_surfpool_cmd(vec!["run", "set_peers", "--unsupervised"])?;
    Ok(())
}

#[test]
fn test_03_check_globals() -> Result<()> {
    let client = crate::get_rpc_client();

    let data_wh = client.get_account_data(&pda!(&[b"global"], &wormhole_adapter::ID))?;
    let data_hyp = client.get_account_data(&pda!(&[b"global"], &hyperlane_adapter::ID))?;

    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;

    assert_eq!(global_wh.peers.len(), 2);
    assert_eq!(global_hp.peers.len(), 1);

    assert_eq!(
        global_wh.peers[0].address,
        [
            11, 134, 236, 24, 28, 212, 197, 201, 132, 233, 6, 43, 19, 242, 178, 222, 123, 159, 91,
            94, 104, 232, 67, 73, 35, 29, 102, 20, 205, 243, 249, 159,
        ]
    );
    assert_eq!(
        global_hp.peers[0].address,
        [
            11, 134, 236, 24, 28, 212, 197, 201, 132, 233, 6, 43, 19, 242, 178, 222, 123, 159, 91,
            94, 104, 232, 67, 73, 35, 29, 102, 20, 205, 243, 249, 159,
        ]
    );

    Ok(())
}
