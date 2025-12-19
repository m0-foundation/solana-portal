use anchor_client::{Client, Cluster};
use anchor_lang::{prelude::Pubkey, system_program, AccountDeserialize};
use anchor_spl::token_2022;
use anyhow::Result;
use common::hyperlane_adapter::accounts::HyperlaneGlobal;
use common::hyperlane_adapter::client::args::AcceptAdmin;
use common::{
    ext_swap::{self, accounts::SwapGlobal},
    hyperlane_adapter, pda,
    portal::constants::{GLOBAL_SEED, MINT_AUTHORITY_SEED, M_VAULT_SEED},
    wormhole_adapter::{self},
    HyperlaneRemainingAccounts, WormholeRemainingAccounts, AUTHORITY_SEED,
};
use solana_sdk::account;
use solana_sdk::feature_set::add_get_minimum_delegation_instruction_to_stake_program;
use hex;
use std::sync::Arc;

use portal::{accounts as portal_accounts, instruction as portal_instruction};
use std::str::FromStr;

use crate::{get_rpc_client, get_signer, run_surfpool_cmd};
use solana_sdk::signature::Keypair;

#[test]
fn test_01_send_token_wormhole() -> Result<()> {
    let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
    let rpc_client: Arc<solana_client::rpc_client::RpcClient> = get_rpc_client();

    let program = client.program(portal::ID)?;
    let m_mint = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH").unwrap();
    let extension_mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp").unwrap();
    let extension_program =
        Pubkey::from_str("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko").unwrap();

    let m_token_account = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &pda!(&[AUTHORITY_SEED], &portal::ID),
        &m_mint,
    )?;
    let extension_token_account = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &program.payer(),
        &extension_mint,
    )?;
    let ext_m_vault = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &pda!(&[M_VAULT_SEED], &extension_program),
        &m_mint,
    )?;

    // Fetch, modify, and print ext_swap::SwapGlobal account data with admin set to program.payer
    let swap_global_pk = pda!(&[GLOBAL_SEED], &ext_swap::ID); 
    let mut swap_data = rpc_client.get_account_data(&swap_global_pk)?;
    let swap_global_acc = SwapGlobal::try_deserialize(&mut swap_data.as_slice())?;

    // Anchor account layout: 8-byte discriminator, then fields per Borsh. Fields: u8 bump, Pubkey admin, ...
    let admin_offset = 8 + 1; // discriminator + bump
    if swap_data.len() < admin_offset + 32 {
        panic!(
            "swap_global data too short ({} bytes), cannot set admin",
            swap_data.len()
        );
    }
    // Read current admin for logging
    // let mut admin_before_arr = [0u8; 32];
    // admin_before_arr.copy_from_slice(&swap_data[admin_offset..admin_offset + 32]);
    // let admin_before = Pubkey::new_from_array(admin_before_arr);

    // Overwrite admin with program.payer
    let payer_bytes = program.payer().to_bytes();
    swap_data[admin_offset..admin_offset + 32].copy_from_slice(&payer_bytes);

    // Hex-encode full account bytes for runbook surfnet_setAccount "data" field
    let swap_data_hex = format!("0x{}", hex::encode(&swap_data));

    let logs = run_surfpool_cmd(vec!["run", "authorize_unwrapper", "--unsupervised"])?;
    println!("Logs from authorize_unwrapper runbook: {}", logs);

    // assert!(!logs.contains("error"), "Funding failed: {}", logs);

    println!("swap_global program: {}", ext_swap::ID);
    println!("swap_global: {}", swap_global_pk);
    println!("test_admin: {}", program.payer());
    println!("admin (before): {}", swap_global_acc.admin);

    let changed_swap_data = rpc_client.get_account_data(&swap_global_pk)?;
    let changed_swap_global_acc = SwapGlobal::try_deserialize(&mut changed_swap_data.as_slice())?;
    println!("admin (after): {}", changed_swap_global_acc.admin);

    println!(
        "swap_global data (admin set to payer) for runbook: {}",
        swap_data_hex
    );
    
    let changed_swap_data = rpc_client.get_account_data(&swap_global_pk)?;
    let changed_swap_global_acc = SwapGlobal::try_deserialize(&mut changed_swap_data.as_slice())?;

    assert_eq!(changed_swap_global_acc.admin, program.payer());


    // Send token update
    let err = program
        .request()
        .accounts(portal_accounts::SendToken {
            sender: program.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            extension_global: pda!(&[GLOBAL_SEED], &extension_program),
            m_mint,
            extension_mint,
            m_token_account,
            extension_token_account,
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            ext_m_vault,
            ext_m_vault_auth: pda!(&[M_VAULT_SEED], &extension_program),
            ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &extension_program),
            swap_program: ext_swap::ID,
            extension_program,
            m_token_program: token_2022::ID,
            extension_token_program: token_2022::ID,
            bridge_adapter: wormhole_adapter::ID,
            system_program: system_program::ID,
        })
        .args(portal_instruction::SendToken {
            amount: 1_000_000,
            destination_token: m_mint.to_bytes(),
            destination_chain_id: 2,
            recipient: program.payer().to_bytes(),
        })
        .accounts(WormholeRemainingAccounts::account_metas())
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("6003") || s.contains("custom program error: 0x1778"));
    assert!(s.contains("UnauthorizedUnwrapper"));

    Ok(())
}

#[test]
fn test_02_send_token_wormhole_unauthorized_unwrapper() -> Result<()> {
    let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
    let rpc_client: Arc<solana_client::rpc_client::RpcClient> = get_rpc_client();

    let program = client.program(portal::ID)?;
    let m_mint = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH").unwrap();
    let extension_mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp").unwrap();
    let extension_program =
        Pubkey::from_str("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko").unwrap();

    let m_token_account = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &pda!(&[AUTHORITY_SEED], &portal::ID),
        &m_mint,
    )?;
    let extension_token_account = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &program.payer(),
        &extension_mint,
    )?;
    let ext_m_vault = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &pda!(&[M_VAULT_SEED], &extension_program),
        &m_mint,
    )?;

    // Send token update
    let err = program
        .request()
        .accounts(portal_accounts::SendToken {
            sender: program.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            extension_global: pda!(&[GLOBAL_SEED], &extension_program),
            m_mint,
            extension_mint,
            m_token_account,
            extension_token_account,
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            ext_m_vault,
            ext_m_vault_auth: pda!(&[M_VAULT_SEED], &extension_program),
            ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &extension_program),
            swap_program: ext_swap::ID,
            extension_program,
            m_token_program: token_2022::ID,
            extension_token_program: token_2022::ID,
            bridge_adapter: wormhole_adapter::ID,
            system_program: system_program::ID,
        })
        .args(portal_instruction::SendToken {
            amount: 1_000_000,
            destination_token: m_mint.to_bytes(),
            destination_chain_id: 2,
            recipient: program.payer().to_bytes(),
        })
        .accounts(WormholeRemainingAccounts::account_metas())
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("6003") || s.contains("custom program error: 0x1778"));
    assert!(s.contains("UnauthorizedUnwrapper"));

    Ok(())
}

#[test]
fn test_03_send_token_hyperlane_unauthorized_unwrapper() -> Result<()> {
    let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
    let rpc_client: Arc<solana_client::rpc_client::RpcClient> = get_rpc_client();

    let program = client.program(portal::ID)?;
    let m_mint = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH").unwrap();
    let extension_mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp").unwrap();
    let extension_program =
        Pubkey::from_str("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko").unwrap();

    let m_token_account = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &pda!(&[AUTHORITY_SEED], &portal::ID),
        &m_mint,
    )?;
    let extension_token_account = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &program.payer(),
        &extension_mint,
    )?;
    let ext_m_vault = crate::util::tokens::get_or_create_ata_2022(
        &rpc_client,
        &get_signer(),
        &pda!(&[M_VAULT_SEED], &extension_program),
        &m_mint,
    )?;

    // Build Hyperlane remaining accounts from on-chain global
    let data_hyp = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID))?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;
    let hyp_accounts = HyperlaneRemainingAccounts::new(&program.payer(), &global_hp, None);

    // Send token update via Hyperlane adapter with remaining accounts
    let err = program
        .request()
        .accounts(portal_accounts::SendToken {
            sender: program.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            extension_global: pda!(&[GLOBAL_SEED], &extension_program),
            m_mint,
            extension_mint,
            m_token_account,
            extension_token_account,
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            ext_m_vault,
            ext_m_vault_auth: pda!(&[M_VAULT_SEED], &extension_program),
            ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &extension_program),
            swap_program: ext_swap::ID,
            extension_program,
            m_token_program: token_2022::ID,
            extension_token_program: token_2022::ID, // Token-2022
            bridge_adapter: hyperlane_adapter::ID,
            system_program: system_program::ID,
        })
        .args(portal_instruction::SendToken {
            amount: 1_000_000,
            destination_token: m_mint.to_bytes(),
            destination_chain_id: 2,
            recipient: program.payer().to_bytes(),
        })
        .accounts(hyp_accounts.to_account_metas())
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("6003") || s.contains("custom program error: 0x1778"));
    assert!(s.contains("UnauthorizedUnwrapper"));

    Ok(())
}
