use anchor_client::{Client, Cluster, Program};
use anchor_lang::{prelude::Pubkey, system_program, AccountDeserialize};
use anyhow::{Ok, Result};
use m0_portal_common::{
    pda,
    portal::constants::{CHAIN_PATHS_SEED, GLOBAL_SEED},
};
use portal::{
    accounts as portal_accounts, instruction as portal_instruction, state::ChainBridgePaths,
};
use solana_sdk::signature::Keypair;
use std::{str::FromStr, sync::Arc};

use crate::{get_rpc_client, get_signer, run_surfpool_cmd};

#[test]
fn test_01_add_path() -> Result<()> {
    let logs = run_surfpool_cmd(vec!["run", "set_path", "--unsupervised"])?;
    assert!(!logs.contains("error"), "Set path failed: {}", logs);

    let client = get_rpc_client();
    let chain_id = 1u32.to_be_bytes();
    let data = client.get_account_data(&pda!(&[CHAIN_PATHS_SEED, &chain_id], &portal::ID))?;
    let paths = ChainBridgePaths::try_deserialize(&mut data.as_slice())?;

    assert_eq!(paths.destination_chain_id, 1);
    assert_eq!(paths.paths.len(), 1);
    assert_eq!(
        paths.paths[0].source_mint.to_string(),
        "mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp"
    );
    assert!(hex::encode(paths.paths[0].destination_token)
        .trim_start_matches("0")
        .eq_ignore_ascii_case("437cc33344a0B27A429f795ff6B469C72698B291"));

    Ok(())
}

#[test]
fn test_02_path_other_chain() -> Result<()> {
    let program = portal_program();
    let chain_id = 42161u32.to_be_bytes();

    program
        .request()
        .accounts(portal_accounts::AddBridgePath {
            admin: program.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            chain_paths: pda!(&[CHAIN_PATHS_SEED, &chain_id], &portal::ID),
            system_program: system_program::ID,
        })
        .args(portal_instruction::AddBridgePath {
            destination_chain_id: 42161,
            path: portal::state::BridgePath {
                source_mint: Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp")?,
                destination_token: hex::decode(
                    "000000000000000000000000437cc33344a0b27a429f795ff6b469c72698b291",
                )?
                .try_into()
                .unwrap(),
            },
        })
        .send()?;

    let client = get_rpc_client();
    let data = client.get_account_data(&pda!(&[CHAIN_PATHS_SEED, &chain_id], &portal::ID))?;
    let paths = ChainBridgePaths::try_deserialize(&mut data.as_slice())?;

    assert_eq!(paths.destination_chain_id, 42161);
    assert_eq!(paths.paths.len(), 1);
    assert_eq!(
        paths.paths[0].source_mint.to_string(),
        "mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp"
    );
    assert!(hex::encode(paths.paths[0].destination_token)
        .trim_start_matches("0")
        .eq_ignore_ascii_case("437cc33344a0B27A429f795ff6B469C72698B291"));

    Ok(())
}
#[test]
fn test_03_path_already_exists() -> Result<()> {
    let err = run_surfpool_cmd(vec!["run", "set_path", "--unsupervised"]).unwrap_err();

    assert!(
        err.to_string().contains("PathAlreadyExists"),
        "Expected initialization failure: {}",
        err
    );

    Ok(())
}

#[test]
fn test_04_add_second_bridge_path() -> Result<()> {
    let program = portal_program();
    let chain_id = 1u32.to_be_bytes();

    program
        .request()
        .accounts(portal_accounts::AddBridgePath {
            admin: program.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            chain_paths: pda!(&[CHAIN_PATHS_SEED, &chain_id], &portal::ID),
            system_program: system_program::ID,
        })
        .args(portal_instruction::AddBridgePath {
            destination_chain_id: 1,
            path: portal::state::BridgePath {
                source_mint: Pubkey::from_str("usdkbee86pkLyRmxfFCdkyySpxRb5ndCxVsK2BkRXwX")?,
                destination_token: hex::decode(
                    "000000000000000000000000437cc33344a0b27a429f795ff6b469c72698b291",
                )?
                .try_into()
                .unwrap(),
            },
        })
        .send()?;

    let client = get_rpc_client();
    let data = client.get_account_data(&pda!(&[CHAIN_PATHS_SEED, &chain_id], &portal::ID))?;
    let paths = ChainBridgePaths::try_deserialize(&mut data.as_slice())?;

    assert_eq!(paths.destination_chain_id, 1);
    assert_eq!(paths.paths.len(), 2);
    assert_eq!(
        paths.paths[1].source_mint.to_string(),
        "usdkbee86pkLyRmxfFCdkyySpxRb5ndCxVsK2BkRXwX"
    );
    assert!(hex::encode(paths.paths[0].destination_token)
        .trim_start_matches("0")
        .eq_ignore_ascii_case("437cc33344a0B27A429f795ff6B469C72698B291"));

    Ok(())
}

#[test]
fn test_05_remove_bridge_path_success() -> Result<()> {
    let program = portal_program();
    let chain_id: u32 = 1;

    // Remove the second path we added
    program
        .request()
        .accounts(portal_accounts::RemoveBridgePath {
            admin: program.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            chain_paths: pda!(&[CHAIN_PATHS_SEED, &chain_id.to_be_bytes()], &portal::ID),
            system_program: system_program::ID,
        })
        .args(portal_instruction::RemoveBridgePath {
            destination_chain_id: chain_id,
            path: portal::state::BridgePath {
                source_mint: Pubkey::from_str("usdkbee86pkLyRmxfFCdkyySpxRb5ndCxVsK2BkRXwX")?,
                destination_token: hex::decode(
                    "000000000000000000000000437cc33344a0b27a429f795ff6b469c72698b291",
                )?
                .try_into()
                .unwrap(),
            },
        })
        .send()?;

    // Verify the path was removed
    let client = get_rpc_client();
    let chain_id = 1u32.to_be_bytes();
    let data = client.get_account_data(&pda!(&[CHAIN_PATHS_SEED, &chain_id], &portal::ID))?;
    let paths = ChainBridgePaths::try_deserialize(&mut data.as_slice())?;

    assert_eq!(paths.paths.len(), 1);

    // Verify only the first path remains
    assert_eq!(
        paths.paths[0].source_mint.to_string(),
        "mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp"
    );
    assert!(hex::encode(paths.paths[0].destination_token)
        .trim_start_matches("0")
        .eq_ignore_ascii_case("437cc33344a0B27A429f795ff6B469C72698B291"));

    Ok(())
}

#[test]
fn test_06_remove_bridge_path_not_found() -> Result<()> {
    let destination_chain_id: u32 = 1;
    let program = portal_program();

    // Try to remove a path that doesn't exist
    let err = program
        .request()
        .accounts(portal_accounts::RemoveBridgePath {
            admin: program.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            chain_paths: pda!(
                &[CHAIN_PATHS_SEED, &destination_chain_id.to_be_bytes()],
                &portal::ID
            ),
            system_program: system_program::ID,
        })
        .args(portal_instruction::RemoveBridgePath {
            destination_chain_id,
            path: portal::state::BridgePath {
                source_mint: Pubkey::new_unique(), // Non-existent path
                destination_token: [0u8; 32],
            },
        })
        .send()
        .unwrap_err();

    assert_err_contains(err, &["PathNotFound"]);

    Ok(())
}

fn portal_program() -> Program<Arc<Keypair>> {
    let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
    client.program(portal::ID).unwrap()
}

fn assert_err_contains(err: impl ToString, substrings: &[&str]) {
    let s = err.to_string();
    for substring in substrings {
        assert!(s.contains(substring), "Expected '{}' in: {}", substring, s);
    }
}
