use anchor_client::{Client, Cluster};
use anyhow::Result;
use common::portal;

use crate::VALIDATOR;

#[test]
fn test_01_index_update_wormhole() -> Result<()> {
    let validator = VALIDATOR.lock().unwrap();
    let client = Client::new(Cluster::Localnet, validator.keypair.clone());

    let program = client.program(portal::ID)?;

    // let result = program
    //     .request()
    //     .accounts(accounts::Initialize {
    //         my_account: my_account_kp.pubkey(),
    //         payer: program.payer(),
    //         system_program: system_program::ID,
    //     })
    //     .args(instruction::Initialize { field: 42 })
    //     .signer(&my_account_kp)
    //     .send();

    Ok(())
}

#[test]
fn test_02_index_update_hyperlane() -> Result<()> {
    Ok(())
}
