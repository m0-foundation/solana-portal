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
    HyperlaneRemainingAccounts, PayloadData, WormholeRemainingAccounts, AUTHORITY_SEED,
    m_ext::{self, accounts::ExtGlobalV2}
};
use solana_sdk::account;
use solana_sdk::feature_set::add_get_minimum_delegation_instruction_to_stake_program;
use hex;
use std::sync::Arc;

use portal::{accounts as portal_accounts, instruction as portal_instruction, state::PortalGlobal};
use std::str::FromStr;

use crate::{get_rpc_client, get_signer, run_surfpool_cmd, util};
use solana_transaction_status_client_types::UiTransactionEncoding;
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
    let m_ext_program =
        Pubkey::from_str("3C865D264L4NkAm78zfnDzQJJvXuU3fMjRUvRxyPi5da").unwrap();

    // --- Log existence of key programs on local chain ---
    let program_ids = vec![
        ("portal", portal::ID),
        ("ext_swap", ext_swap::ID),
        ("extension_program", extension_program),
        ("m_ext_program", m_ext_program),
    ];
    for (name, pk) in program_ids {
        match rpc_client.get_account(&pk) {
            Ok(acc) => println!("Program {name} ({pk}) exists, owner: {}", acc.owner),
            Err(e) => println!("Program {name} ({pk}) missing: {e}"),
        }
    }
    assert!(1 == 0); // Dummy assertion to mark test as passed for now

    // let m_token_account = crate::util::tokens::get_or_create_ata_2022(
    //     &rpc_client,
    //     &get_signer(),
    //     &pda!(&[AUTHORITY_SEED], &portal::ID),
    //     &m_mint,
    // )?;
    // let extension_token_account = crate::util::tokens::get_or_create_ata_2022(
    //     &rpc_client,
    //     &get_signer(),
    //     &program.payer(),
    //     &extension_mint,
    // )?;
    // let ext_m_vault = crate::util::tokens::get_or_create_ata_2022(
    //     &rpc_client,
    //     &get_signer(),
    //     &pda!(&[M_VAULT_SEED], &extension_program),
    //     &m_mint,
    // )?;

    // // Fetch, modify, and print ext_swap::SwapGlobal account data with admin set to program.payer
    // let swap_global_pk = pda!(&[GLOBAL_SEED], &ext_swap::ID); 
    // let mut swap_data = rpc_client.get_account_data(&swap_global_pk)?;

    // // Anchor account layout: 8-byte discriminator, then fields per Borsh. Fields: u8 bump, Pubkey admin, ...
    // let admin_offset = 8 + 1; // discriminator + bump

    // // Overwrite admin with program.payer
    // let payer_bytes = program.payer().to_bytes();
    // swap_data[admin_offset..admin_offset + 32].copy_from_slice(&payer_bytes);

    // println!("PRE get data m_ext data");



//    // Fetch, modify, and print m_ext::ExtGlobalV2 account data with admin set to program.payer
//     let m_ext_global_pk = pda!(&[b"global"], &m_ext_program);
//     let mut extension_data: Vec<u8> = rpc_client.get_account_data(&m_ext_global_pk)?;
//     println!("PRE deserialize extension_global pk: {:?}", m_ext_global_pk);

//     let changed_ext_global_acc = ExtGlobalV2::try_deserialize(&mut extension_data.as_slice())?;
//     println!("extension data: {:?}", changed_ext_global_acc);
    

//     // ExtGlobal layout: 8-byte discriminator, then admin (32 bytes)
//     let ext_admin_offset = 8;
//     if extension_data.len() < ext_admin_offset + 32 {
//         panic!(
//             "extension_global data too short ({} bytes), cannot set admin",
//             extension_data.len()
//         );
//     }

//     // Overwrite admin with program.payer
//     let payer_bytes = program.payer().to_bytes();
//     extension_data[ext_admin_offset..ext_admin_offset + 32].copy_from_slice(&payer_bytes);

//     // Print hex for runbook usage
//     // println!("extension_global data after admin set to payer: {}", hex::encode(&extension_data));



//     // authorize swap facility admin to be program.payer() so it can whitelist the portal authority as an unwrapper
//     let logs = run_surfpool_cmd(vec!["run", "authorize_unwrapper", "--unsupervised"])?;
//     assert!(!logs.contains("error"), "Funding failed: {}", logs); 

//     let changed_swap_data = rpc_client.get_account_data(&swap_global_pk)?;
//     // println!("swap_global data after runbook: {}", hex::encode(&changed_swap_data));
//     let changed_swap_global_acc = SwapGlobal::try_deserialize(&mut changed_swap_data.as_slice())?;

//     let changed_ext_data = rpc_client.get_account_data(&m_ext_global_pk)?;
//     // println!("extension_global data after runbook: {}", hex::encode(&changed_ext_data));
//     let changed_ext_global_acc = ExtGlobalV2::try_deserialize(&mut changed_ext_data.as_slice())?;
//     println!("deserialize: {}", changed_ext_global_acc.admin);

//     assert_eq!(changed_swap_global_acc.admin, program.payer());
//     println!("swap_global admin (after): {}", changed_swap_global_acc.admin);
//     assert_eq!(changed_ext_global_acc.admin, program.payer());

//     let swap_program = client.program(ext_swap::ID)?;
    
//     let signature = swap_program
//         .request()
//         .accounts(ext_swap::client::accounts::WhitelistUnwrapper {
//             admin: program.payer(),
//             swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
//             system_program: system_program::ID,
//         })
//         .args(ext_swap::client::args::WhitelistUnwrapper {
//             authority: pda!(&[AUTHORITY_SEED], &portal::ID),
//         })
//         .send()?;

//     // Fetch, modify, and print ext_swap::SwapGlobal account data with admin set to program.payer
//     let swap_global_pk = pda!(&[GLOBAL_SEED], &ext_swap::ID); 
//     let swap_data = rpc_client.get_account_data(&swap_global_pk)?;
//     let swap_acc = SwapGlobal::try_deserialize(&mut swap_data.as_slice())?;
//     let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);
//     assert!(
//         swap_acc.whitelisted_unwrappers.contains(&portal_authority),
//         "Portal authority {} is not whitelisted as an unwrapper",
//         portal_authority
//     );



    
//     // // Send token update
//     // let signature = program
//     //     .request()
//     //     .accounts(portal_accounts::SendToken {
//     //         sender: program.payer(),
//     //         portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
//     //         swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
//     //         extension_global: pda!(&[GLOBAL_SEED], &extension_program),
//     //         m_mint,
//     //         extension_mint,
//     //         m_token_account,
//     //         extension_token_account,
//     //         portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
//     //         ext_m_vault,
//     //         ext_m_vault_auth: pda!(&[M_VAULT_SEED], &extension_program),
//     //         ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &extension_program),
//     //         swap_program: ext_swap::ID,
//     //         extension_program,
//     //         m_token_program: token_2022::ID,
//     //         extension_token_program: token_2022::ID,
//     //         bridge_adapter: wormhole_adapter::ID,
//     //         system_program: system_program::ID,
//     //     })
//     //     .args(portal_instruction::SendToken {
//     //         amount: 1_000_000,
//     //         destination_token: m_mint.to_bytes(),
//     //         destination_chain_id: 2,
//     //         recipient: program.payer().to_bytes(),
//     //     })
//     //     .accounts(WormholeRemainingAccounts::account_metas())
//     //     .send()?;

    // let transaction = rpc_client.get_transaction(&signature, UiTransactionEncoding::Json)?;

    // let payload =
    //     util::wormhole::find_post_message_payload(&transaction).expect("Index payload not found");

    // // tokenTransferPayload should match what we sent
    // let portal_global_bytes = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &portal::ID))?;
    // let portal_global = PortalGlobal::try_deserialize(&mut portal_global_bytes.as_slice())?;

    // match payload.data {
    //     PayloadData::TokenTransfer(token_payload) => {
    //         assert_eq!(token_payload.index, portal_global.m_index);
    //         assert_eq!(token_payload.amount, 1_000_000);
    //         assert_eq!(token_payload.destination_token, m_mint.to_bytes());
    //         assert_eq!(token_payload.sender, program.payer().to_bytes());
    //         assert_eq!(token_payload.recipient, program.payer().to_bytes());
    //     }
    //     _ => panic!("Expected TokenTransferPayload"),
    // }

    Ok(())
}

// #[test]
// fn test_02_send_token_wormhole_unauthorized_unwrapper() -> Result<()> {
//     let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
//     let rpc_client: Arc<solana_client::rpc_client::RpcClient> = get_rpc_client();

//     let program = client.program(portal::ID)?;
//     let m_mint = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH").unwrap();
//     let extension_mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp").unwrap();
//     let extension_program =
//         Pubkey::from_str("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko").unwrap();

//     let m_token_account = crate::util::tokens::get_or_create_ata_2022(
//         &rpc_client,
//         &get_signer(),
//         &pda!(&[AUTHORITY_SEED], &portal::ID),
//         &m_mint,
//     )?;
//     let extension_token_account = crate::util::tokens::get_or_create_ata_2022(
//         &rpc_client,
//         &get_signer(),
//         &program.payer(),
//         &extension_mint,
//     )?;
//     let ext_m_vault = crate::util::tokens::get_or_create_ata_2022(
//         &rpc_client,
//         &get_signer(),
//         &pda!(&[M_VAULT_SEED], &extension_program),
//         &m_mint,
//     )?;

//     // Send token update
//     let err = program
//         .request()
//         .accounts(portal_accounts::SendToken {
//             sender: program.payer(),
//             portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
//             swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
//             extension_global: pda!(&[GLOBAL_SEED], &extension_program),
//             m_mint,
//             extension_mint,
//             m_token_account,
//             extension_token_account,
//             portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
//             ext_m_vault,
//             ext_m_vault_auth: pda!(&[M_VAULT_SEED], &extension_program),
//             ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &extension_program),
//             swap_program: ext_swap::ID,
//             extension_program,
//             m_token_program: token_2022::ID,
//             extension_token_program: token_2022::ID,
//             bridge_adapter: wormhole_adapter::ID,
//             system_program: system_program::ID,
//         })
//         .args(portal_instruction::SendToken {
//             amount: 1_000_000,
//             destination_token: m_mint.to_bytes(),
//             destination_chain_id: 2,
//             recipient: program.payer().to_bytes(),
//         })
//         .accounts(WormholeRemainingAccounts::account_metas())
//         .send()
//         .unwrap_err();

//     let s = err.to_string();
//     assert!(s.contains("6003") || s.contains("custom program error: 0x1778"));
//     assert!(s.contains("UnauthorizedUnwrapper"));

//     Ok(())
// }

// #[test]
// fn test_03_send_token_hyperlane_unauthorized_unwrapper() -> Result<()> {
//     let client: Client<Arc<Keypair>> = Client::new(Cluster::Localnet, get_signer());
//     let rpc_client: Arc<solana_client::rpc_client::RpcClient> = get_rpc_client();

//     let program = client.program(portal::ID)?;
//     let m_mint = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH").unwrap();
//     let extension_mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp").unwrap();
//     let extension_program =
//         Pubkey::from_str("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko").unwrap();

//     let m_token_account = crate::util::tokens::get_or_create_ata_2022(
//         &rpc_client,
//         &get_signer(),
//         &pda!(&[AUTHORITY_SEED], &portal::ID),
//         &m_mint,
//     )?;
//     let extension_token_account = crate::util::tokens::get_or_create_ata_2022(
//         &rpc_client,
//         &get_signer(),
//         &program.payer(),
//         &extension_mint,
//     )?;
//     let ext_m_vault = crate::util::tokens::get_or_create_ata_2022(
//         &rpc_client,
//         &get_signer(),
//         &pda!(&[M_VAULT_SEED], &extension_program),
//         &m_mint,
//     )?;

//     // Build Hyperlane remaining accounts from on-chain global
//     let data_hyp = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID))?;
//     let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;
//     let hyp_accounts = HyperlaneRemainingAccounts::new(&program.payer(), &global_hp, None);

//     // Send token update via Hyperlane adapter with remaining accounts
//     let err = program
//         .request()
//         .accounts(portal_accounts::SendToken {
//             sender: program.payer(),
//             portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
//             swap_global: pda!(&[GLOBAL_SEED], &ext_swap::ID),
//             extension_global: pda!(&[GLOBAL_SEED], &extension_program),
//             m_mint,
//             extension_mint,
//             m_token_account,
//             extension_token_account,
//             portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
//             ext_m_vault,
//             ext_m_vault_auth: pda!(&[M_VAULT_SEED], &extension_program),
//             ext_mint_authority: pda!(&[MINT_AUTHORITY_SEED], &extension_program),
//             swap_program: ext_swap::ID,
//             extension_program,
//             m_token_program: token_2022::ID,
//             extension_token_program: token_2022::ID, // Token-2022
//             bridge_adapter: hyperlane_adapter::ID,
//             system_program: system_program::ID,
//         })
//         .args(portal_instruction::SendToken {
//             amount: 1_000_000,
//             destination_token: m_mint.to_bytes(),
//             destination_chain_id: 2,
//             recipient: program.payer().to_bytes(),
//         })
//         .accounts(hyp_accounts.to_account_metas())
//         .send()
//         .unwrap_err();

//     let s = err.to_string();
//     assert!(s.contains("6003") || s.contains("custom program error: 0x1778"));
//     assert!(s.contains("UnauthorizedUnwrapper"));

//     Ok(())
// }
