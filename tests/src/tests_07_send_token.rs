use anchor_client::{Client, Cluster};
use anchor_lang::{prelude::Pubkey, system_program, AccountDeserialize};
use anchor_spl::token_2022;
use anyhow::Result;
use common::hyperlane_adapter::accounts::HyperlaneGlobal;
use common::hyperlane_adapter::client::args::AcceptAdmin;
use common::{
    ext_swap::{self, accounts::SwapGlobal},
    hyperlane_adapter,
    m_ext::{self, accounts::ExtGlobalV2},
    pda,
    portal::constants::{GLOBAL_SEED, MINT_AUTHORITY_SEED, M_VAULT_SEED},
    wormhole_adapter::{self},
    HyperlaneRemainingAccounts, PayloadData, WormholeRemainingAccounts, AUTHORITY_SEED,
};
use hex;
use solana_sdk::account;
use solana_sdk::feature_set::add_get_minimum_delegation_instruction_to_stake_program;
use std::sync::Arc;

use portal::{accounts as portal_accounts, instruction as portal_instruction, state::PortalGlobal};
use std::str::FromStr;

use crate::{get_rpc_client, get_signer, run_surfpool_cmd, util};
use solana_sdk::signature::Keypair;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_transaction_status_client_types::UiTransactionEncoding;
use spl_token_2022::{
    state::{Account as TokenAccount2022, AccountState},
    extension::StateWithExtensions
};


#[test]
fn test_01_send_token_wormhole_unauthorized_unwrapper() -> Result<()> {
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
fn test_02_send_token_hyperlane_unauthorized_unwrapper() -> Result<()> {
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


#[test]
fn test_03_send_token_wormhole_insufficient_funds() -> Result<()> {
    let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
    let rpc_client: Arc<solana_client::rpc_client::RpcClient> = get_rpc_client();

    let program = client.program(portal::ID)?;
    let m_mint: Pubkey = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH").unwrap();
    let extension_mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp").unwrap();
    let ext_program_id = Pubkey::from_str("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko").unwrap();
    let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);

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
        &pda!(&[M_VAULT_SEED], &ext_program_id),
        &m_mint,
    )?;


    // authorize swap facility admin to be program.payer() so it can whitelist the portal authority as an unwrapper
    let logs = run_surfpool_cmd(vec!["run", "authorize_unwrapper", "--unsupervised"])?;
    assert!(!logs.contains("error"), "Funding failed: {}", logs);

    let swap_global_pk = pda!(&[GLOBAL_SEED], &ext_swap::ID);
    let changed_swap_data = rpc_client.get_account_data(&swap_global_pk)?;
    let changed_swap_global_acc = SwapGlobal::try_deserialize(&mut changed_swap_data.as_slice())?;

    let m_ext_global_pk = pda!(&[GLOBAL_SEED], &ext_program_id);
    let changed_ext_data = rpc_client.get_account_data(&m_ext_global_pk)?;
    let changed_ext_global_acc = ExtGlobalV2::try_deserialize(&mut changed_ext_data.as_slice())?;

    assert_eq!(changed_swap_global_acc.admin, program.payer());
    assert_eq!(changed_ext_global_acc.admin, program.payer());

    let swap_program = client.program(ext_swap::ID)?;

    let signature = swap_program
        .request()
        .accounts(ext_swap::client::accounts::WhitelistUnwrapper {
            admin: program.payer(),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            system_program: system_program::ID,
        })
        .args(ext_swap::client::args::WhitelistUnwrapper {
            authority: portal_authority,
        })
        .send()?;

    // Fetch, modify, and print ext_swap::SwapGlobal account data with admin set to program.payer
    let swap_global_pk = pda!(&[GLOBAL_SEED], &ext_swap::ID);
    let swap_data = rpc_client.get_account_data(&swap_global_pk)?;
    let swap_acc = SwapGlobal::try_deserialize(&mut swap_data.as_slice())?;
    let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);
    assert!(
        swap_acc.whitelisted_unwrappers.contains(&portal_authority),
        "Portal authority {} is not whitelisted as an unwrapper",
        portal_authority
    );

    let extension_program = client.program(ext_program_id)?;

    let signature = extension_program
        .request()
        .accounts(m_ext::client::accounts::AddWrapAuthority {
            admin: program.payer(),
            global_account: pda!(&[GLOBAL_SEED], &ext_program_id),
            system_program: system_program::ID,
        })
        .args(m_ext::client::args::AddWrapAuthority {
            new_wrap_authority: portal_authority,
        })
        .send()?;

    // Fetch, modify, and print ext_swap::SwapGlobal account data with admin set to program.payer
    let ext_data = rpc_client.get_account_data(&m_ext_global_pk)?;
    let ext_acc = ExtGlobalV2::try_deserialize(&mut ext_data.as_slice())?;

    assert!(
        ext_acc.wrap_authorities.contains(&portal_authority),
        "Portal authority {} is not whitelisted as an wrap authority",
        portal_authority
    );

    // Send token update
    let result = program
        .request()
        .accounts(portal_accounts::SendToken {
            sender: program.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            extension_global: pda!(&[GLOBAL_SEED], &ext_program_id),
            m_mint,
            extension_mint,
            m_token_account,
            extension_token_account,
            portal_authority,
            ext_m_vault,
            ext_m_vault_auth: pda!(&[M_VAULT_SEED], &ext_program_id),
            ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &ext_program_id),
            swap_program: ext_swap::ID,
            extension_program: ext_program_id,
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

    let s = result.to_string();
    assert!(
        s.contains("custom program error: 0x1"),
        "Expected custom program error: 0x1, got: {}",
        s
    );
    assert!(
        s.contains("insufficient funds"),
        "Expected 'insufficient funds' in logs, got: {}",
        s
    );

    Ok(())
}

#[test]
fn test_04_send_token_wormhole_success() -> Result<()> {
    let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
    let rpc_client: Arc<solana_client::rpc_client::RpcClient> = get_rpc_client();

    let program = client.program(portal::ID)?;
    let m_mint = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH").unwrap();
    let extension_mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp").unwrap();
    let ext_program_id = Pubkey::from_str("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko").unwrap();
    let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);

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
        &pda!(&[M_VAULT_SEED], &ext_program_id),
        &m_mint,
    )?;

    // fund sender's extension_token_account with wrapped_m tokens and set portal_authority's m_mint ata to unfrozen
    let logs: String = run_surfpool_cmd(vec!["run", "wm_fund_payer", "--unsupervised"])?;
    assert!(!logs.contains("error"), "Funding failed: {}", logs);

    // Assert the sender has wrapped_m balance of 1000000000
    let balance = rpc_client.get_token_account_balance(&extension_token_account)?;

    assert!(
        balance.amount == "1000000000",
        "Expected extension_token_account to have at least 1,000,000,000 wrapped_m, got {}",
        balance.amount
    );

    let acc = rpc_client.get_account(&m_token_account)?;
    let ta = StateWithExtensions::<TokenAccount2022>::unpack(&acc.data)?;
    let frozen = ta.base.state == AccountState::Frozen;

    assert!(
        frozen == false,
        "Expected portal_authority's m_mint ata to be unfrozen"
    );
 
    // Send token update
    let result = program
        .request()
        .accounts(portal_accounts::SendToken {
            sender: program.payer(),
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
            extension_global: pda!(&[GLOBAL_SEED], &ext_program_id),
            m_mint,
            m_token_account,
            extension_mint,
            extension_token_account,
            extension_program: ext_program_id,
            portal_authority,
            ext_m_vault,
            ext_m_vault_auth: pda!(&[M_VAULT_SEED], &ext_program_id),
            ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &ext_program_id),
            swap_program: ext_swap::ID,
            m_token_program: token_2022::ID,
            extension_token_program: token_2022::ID,
            bridge_adapter: wormhole_adapter::ID,
            system_program: system_program::ID,
        })
        .args(portal_instruction::SendToken {
            amount: 1_000_000,
            destination_token: m_mint.to_bytes(),
            destination_chain_id: 1,
            recipient: program.payer().to_bytes(),
        })
        .accounts(WormholeRemainingAccounts::account_metas())
        .send()?;

    let transaction = rpc_client.get_transaction(&result, UiTransactionEncoding::Json)?;

    let payload =
        util::wormhole::find_post_message_payload(&transaction).expect("Send Token payload not found");

    // tokenTransferPayload should match what we sent
    let portal_global_bytes = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &portal::ID))?;
    let portal_global = PortalGlobal::try_deserialize(&mut portal_global_bytes.as_slice())?;

    match payload.data {
        PayloadData::TokenTransfer(token_payload) => {
            assert_eq!(token_payload.index, portal_global.m_index);
            assert_eq!(token_payload.amount, 936_212); // after fees
            assert_eq!(token_payload.destination_token, m_mint.to_bytes());
            assert_eq!(token_payload.sender, program.payer().to_bytes());
            assert_eq!(token_payload.recipient, program.payer().to_bytes());
        }
        _ => panic!("Expected TokenTransferPayload"),
    }

    Ok(())
}

#[test]
fn test_05_send_token_hyperlane_success() -> Result<()> {
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
    // shouldn't fail, but does due to compute limit / 28 bytes over
    let result = program
        .request()
        .instruction(ComputeBudgetInstruction::set_compute_unit_limit(600_000))
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
            destination_chain_id: 1,
            recipient: program.payer().to_bytes(),
        })
        .accounts(hyp_accounts.to_account_metas())
        .send()?; 


    // let message_account = hyp_accounts.dispatched_message;
    // let account_data = rpc_client.get_account_data(&message_account)?;

    // let (payload, recipient) = util::hyperlane::decode_payload_from_message_account(&account_data)
    //     .expect("Failed to decode sendToken payload");

    // // Index should match the program’s current m_index (you send current index)
    // let global_bytes = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &portal::ID))?;
    // let portal_global = PortalGlobal::try_deserialize(&mut global_bytes.as_slice())?;

    // match payload.data {
    //     PayloadData::TokenTransfer(token_payload) => {
    //         assert_eq!(token_payload.index, portal_global.m_index);
    //         assert_eq!(token_payload.amount, 936_212); // after fees
    //         assert_eq!(token_payload.destination_token, m_mint.to_bytes());
    //         assert_eq!(token_payload.sender, program.payer().to_bytes());
    //         assert_eq!(token_payload.recipient, program.payer().to_bytes());
    //     }
    //     _ => panic!("Expected TokenTransferPayload"),
    // }

    // // Recipient should be registered peer
    // assert_eq!(
    //     hex::encode(recipient),
    //     "0b6a86806a0354c82b8f049eb75d9c97e370a6f0c0cfa15f47909c3fe1c8f794"
    // );

    Ok(())
}


