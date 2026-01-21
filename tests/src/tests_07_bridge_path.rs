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
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use std::{str::FromStr, sync::Arc};

use crate::{get_rpc_client, get_signer};

struct BridgePathTestCtx {
    rpc: Arc<RpcClient>,
    portal: Program<Arc<Keypair>>,
    portal_global: Pubkey,
    m_mint: Pubkey,
    extension_mint: Pubkey,
}

impl BridgePathTestCtx {
    fn new() -> Result<Self> {
        let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
        let rpc: Arc<RpcClient> = get_rpc_client();
        let portal = client.program(portal::ID)?;

        let portal_global = pda!(&[GLOBAL_SEED], &portal::ID);
        let m_mint = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH")?;
        let extension_mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp")?;

        Ok(Self {
            rpc,
            portal,
            portal_global,
            m_mint,
            extension_mint,
        })
    }

    fn chain_paths_pda(&self, destination_chain_id: u32) -> Pubkey {
        pda!(
            &[CHAIN_PATHS_SEED, &destination_chain_id.to_be_bytes()],
            &portal::ID
        )
    }
}

fn assert_err_contains(err: impl ToString, substrings: &[&str]) {
    let s = err.to_string();
    for substring in substrings {
        assert!(s.contains(substring), "Expected '{}' in: {}", substring, s);
    }
}

#[test]
fn test_01_initialize_chain_paths_for_chain_1() -> Result<()> {
    let ctx = BridgePathTestCtx::new()?;
    let destination_chain_id: u32 = 1;

    ctx.portal
        .request()
        .accounts(portal_accounts::InitializeBridgePaths {
            admin: ctx.portal.payer(),
            portal_global: ctx.portal_global,
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            system_program: system_program::ID,
        })
        .args(portal_instruction::InitializeBridgePaths {
            destination_chain_id,
        })
        .send()?;

    // Verify the account was created
    let chain_paths_data = ctx
        .rpc
        .get_account_data(&ctx.chain_paths_pda(destination_chain_id))?;
    let chain_paths = ChainBridgePaths::try_deserialize(&mut chain_paths_data.as_slice())?;

    assert_eq!(chain_paths.destination_chain_id, destination_chain_id);
    assert!(chain_paths.paths.is_empty());

    Ok(())
}

#[test]
fn test_02_initialize_chain_paths_for_chain_2() -> Result<()> {
    let ctx = BridgePathTestCtx::new()?;
    let destination_chain_id: u32 = 2;

    ctx.portal
        .request()
        .accounts(portal_accounts::InitializeBridgePaths {
            admin: ctx.portal.payer(),
            portal_global: ctx.portal_global,
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            system_program: system_program::ID,
        })
        .args(portal_instruction::InitializeBridgePaths {
            destination_chain_id,
        })
        .send()?;

    // Verify the account was created
    let chain_paths_data = ctx
        .rpc
        .get_account_data(&ctx.chain_paths_pda(destination_chain_id))?;
    let chain_paths = ChainBridgePaths::try_deserialize(&mut chain_paths_data.as_slice())?;

    assert_eq!(chain_paths.destination_chain_id, destination_chain_id);
    assert!(chain_paths.paths.is_empty());

    Ok(())
}

#[test]
fn test_03_initialize_chain_paths_already_exists() -> Result<()> {
    let ctx = BridgePathTestCtx::new()?;
    let destination_chain_id: u32 = 1; // Already initialized in test_01

    let err = ctx
        .portal
        .request()
        .accounts(portal_accounts::InitializeBridgePaths {
            admin: ctx.portal.payer(),
            portal_global: ctx.portal_global,
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            system_program: system_program::ID,
        })
        .args(portal_instruction::InitializeBridgePaths {
            destination_chain_id,
        })
        .send()
        .unwrap_err();

    // Should fail because account already exists
    let s = err.to_string();
    assert!(
        s.contains("already in use") || s.contains("0x0"),
        "Expected account already exists error, got: {}",
        s
    );

    Ok(())
}

// ============================================================================
// Add Bridge Path Tests
// ============================================================================

#[test]
fn test_04_add_bridge_path_success() -> Result<()> {
    let ctx = BridgePathTestCtx::new()?;
    let destination_chain_id: u32 = 1;

    // Add path: extension_mint -> m_mint on chain 1
    ctx.portal
        .request()
        .accounts(portal_accounts::AddBridgePath {
            admin: ctx.portal.payer(),
            portal_global: ctx.portal_global,
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            system_program: system_program::ID,
        })
        .args(portal_instruction::AddBridgePath {
            destination_chain_id,
            path: portal::state::BridgePath {
                source_mint: ctx.extension_mint,
                destination_token: ctx.m_mint.to_bytes(),
            },
        })
        .send()?;

    // Verify the path was added
    let chain_paths_data = ctx
        .rpc
        .get_account_data(&ctx.chain_paths_pda(destination_chain_id))?;
    let chain_paths = ChainBridgePaths::try_deserialize(&mut chain_paths_data.as_slice())?;

    assert_eq!(chain_paths.paths.len(), 1);
    assert_eq!(chain_paths.paths[0].source_mint, ctx.extension_mint);
    assert_eq!(
        chain_paths.paths[0].destination_token,
        ctx.m_mint.to_bytes()
    );

    Ok(())
}

#[test]
fn test_05_add_bridge_path_for_chain_2() -> Result<()> {
    let ctx = BridgePathTestCtx::new()?;
    let destination_chain_id: u32 = 2;

    // Add path for chain 2 (needed for some error tests in send_token)
    ctx.portal
        .request()
        .accounts(portal_accounts::AddBridgePath {
            admin: ctx.portal.payer(),
            portal_global: ctx.portal_global,
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            system_program: system_program::ID,
        })
        .args(portal_instruction::AddBridgePath {
            destination_chain_id,
            path: portal::state::BridgePath {
                source_mint: ctx.extension_mint,
                destination_token: ctx.m_mint.to_bytes(),
            },
        })
        .send()?;

    // Verify
    let chain_paths_data = ctx
        .rpc
        .get_account_data(&ctx.chain_paths_pda(destination_chain_id))?;
    let chain_paths = ChainBridgePaths::try_deserialize(&mut chain_paths_data.as_slice())?;

    assert_eq!(chain_paths.paths.len(), 1);

    Ok(())
}

#[test]
fn test_06_add_bridge_path_duplicate_rejected() -> Result<()> {
    let ctx = BridgePathTestCtx::new()?;
    let destination_chain_id: u32 = 1;

    // Try to add the same path again
    let err = ctx
        .portal
        .request()
        .accounts(portal_accounts::AddBridgePath {
            admin: ctx.portal.payer(),
            portal_global: ctx.portal_global,
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            system_program: system_program::ID,
        })
        .args(portal_instruction::AddBridgePath {
            destination_chain_id,
            path: portal::state::BridgePath {
                source_mint: ctx.extension_mint,
                destination_token: ctx.m_mint.to_bytes(),
            },
        })
        .send()
        .unwrap_err();

    assert_err_contains(err, &["PathAlreadyExists"]);

    Ok(())
}

#[test]
fn test_07_add_second_bridge_path() -> Result<()> {
    let ctx = BridgePathTestCtx::new()?;
    let destination_chain_id: u32 = 1;

    // Add a different path (m_mint -> extension_mint)
    ctx.portal
        .request()
        .accounts(portal_accounts::AddBridgePath {
            admin: ctx.portal.payer(),
            portal_global: ctx.portal_global,
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            system_program: system_program::ID,
        })
        .args(portal_instruction::AddBridgePath {
            destination_chain_id,
            path: portal::state::BridgePath {
                source_mint: ctx.m_mint,
                destination_token: ctx.extension_mint.to_bytes(),
            },
        })
        .send()?;

    // Verify both paths exist
    let chain_paths_data = ctx
        .rpc
        .get_account_data(&ctx.chain_paths_pda(destination_chain_id))?;
    let chain_paths = ChainBridgePaths::try_deserialize(&mut chain_paths_data.as_slice())?;

    assert_eq!(chain_paths.paths.len(), 2);

    Ok(())
}

// ============================================================================
// Remove Bridge Path Tests
// ============================================================================

#[test]
fn test_08_remove_bridge_path_success() -> Result<()> {
    let ctx = BridgePathTestCtx::new()?;
    let destination_chain_id: u32 = 1;

    // Get current path count
    let chain_paths_data = ctx
        .rpc
        .get_account_data(&ctx.chain_paths_pda(destination_chain_id))?;
    let chain_paths_before = ChainBridgePaths::try_deserialize(&mut chain_paths_data.as_slice())?;
    let count_before = chain_paths_before.paths.len();

    // Remove the second path we added (m_mint -> extension_mint)
    ctx.portal
        .request()
        .accounts(portal_accounts::RemoveBridgePath {
            admin: ctx.portal.payer(),
            portal_global: ctx.portal_global,
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
            system_program: system_program::ID,
        })
        .args(portal_instruction::RemoveBridgePath {
            destination_chain_id,
            path: portal::state::BridgePath {
                source_mint: ctx.m_mint,
                destination_token: ctx.extension_mint.to_bytes(),
            },
        })
        .send()?;

    // Verify the path was removed
    let chain_paths_data = ctx
        .rpc
        .get_account_data(&ctx.chain_paths_pda(destination_chain_id))?;
    let chain_paths_after = ChainBridgePaths::try_deserialize(&mut chain_paths_data.as_slice())?;

    assert_eq!(chain_paths_after.paths.len(), count_before - 1);

    // Verify only the first path remains
    assert_eq!(chain_paths_after.paths[0].source_mint, ctx.extension_mint);
    assert_eq!(
        chain_paths_after.paths[0].destination_token,
        ctx.m_mint.to_bytes()
    );

    Ok(())
}

#[test]
fn test_09_remove_bridge_path_not_found() -> Result<()> {
    let ctx = BridgePathTestCtx::new()?;
    let destination_chain_id: u32 = 1;

    // Try to remove a path that doesn't exist
    let err = ctx
        .portal
        .request()
        .accounts(portal_accounts::RemoveBridgePath {
            admin: ctx.portal.payer(),
            portal_global: ctx.portal_global,
            chain_paths: ctx.chain_paths_pda(destination_chain_id),
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

// ============================================================================
// Verify Chain Paths Are Ready for send_token Tests
// ============================================================================

#[test]
fn test_10_verify_chain_paths_ready() -> Result<()> {
    let ctx = BridgePathTestCtx::new()?;

    // Verify chain 1 has the required path
    let chain_paths_1_data = ctx.rpc.get_account_data(&ctx.chain_paths_pda(1))?;
    let chain_paths_1 = ChainBridgePaths::try_deserialize(&mut chain_paths_1_data.as_slice())?;

    assert!(
        chain_paths_1.is_path_supported(&ctx.extension_mint, &ctx.m_mint.to_bytes()),
        "Chain 1 should have extension_mint -> m_mint path"
    );

    // Verify chain 2 has the required path
    let chain_paths_2_data = ctx.rpc.get_account_data(&ctx.chain_paths_pda(2))?;
    let chain_paths_2 = ChainBridgePaths::try_deserialize(&mut chain_paths_2_data.as_slice())?;

    assert!(
        chain_paths_2.is_path_supported(&ctx.extension_mint, &ctx.m_mint.to_bytes()),
        "Chain 2 should have extension_mint -> m_mint path"
    );

    Ok(())
}
