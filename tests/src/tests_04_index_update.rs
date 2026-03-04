use anchor_client::{Client, Cluster};
use anchor_lang::{system_program, AccountDeserialize};
use anyhow::Result;
use m0_portal_common::{
    earn,
    hyperlane_adapter::{
        self,
        accounts::{HyperlaneGlobal, HyperlaneUserGlobal},
        constants::DASH_SEED,
    },
    pda,
    portal::constants::GLOBAL_SEED,
    wormhole_adapter::{self, constants::EMITTER_SEED},
    HyperlaneRemainingAccounts, PayloadData, WormholeRemainingAccounts, AUTHORITY_SEED,
};
use portal::{
    state::PortalGlobal,
    {accounts, instruction},
};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_transaction_status_client_types::UiTransactionEncoding;

use crate::{get_rpc_client, get_signer, util};

#[test]
fn test_01_index_update_wormhole() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let rpc_client = get_rpc_client();

    let program = client.program(portal::ID)?;

    // Send index update
    let signature = program
        .request()
        .accounts(accounts::SendIndex {
            sender: program.payer(),
            system_program: system_program::ID,
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            bridge_adapter: wormhole_adapter::ID,
        })
        .args(instruction::SendIndex {
            destination_chain_id: 1,
        })
        .accounts(WormholeRemainingAccounts::account_metas(false))
        .send()?;

    let transaction = rpc_client.get_transaction(&signature, UiTransactionEncoding::Json)?;

    let event_meta = util::wormhole::find_message_event(&transaction)
        .expect("Wormhole post message event not found or invalid");

    let expected_emitter = pda!(&[EMITTER_SEED], &wormhole_adapter::ID).to_bytes();
    assert_eq!(event_meta.emitter, expected_emitter);
    assert!(event_meta.sequence > 50);
    assert!(event_meta.timestamp > 0);

    let message_account = WormholeRemainingAccounts::new(false).message_account;
    let account_data = rpc_client.get_account_data(&message_account)?;

    // Emitter chain and address
    assert_eq!(account_data[57..59], [1, 0]);
    assert_eq!(
        account_data[59..91],
        WormholeRemainingAccounts::new(false).emitter.to_bytes()
    );

    let payload =
        util::wormhole::find_post_message_payload(&transaction).expect("Index payload not found");

    // Index should match the program’s current m_index (you send current index)
    let global_bytes = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &portal::ID))?;
    let portal_global = PortalGlobal::try_deserialize(&mut global_bytes.as_slice())?;
    assert_eq!(payload.header.index, portal_global.m_index);
    assert!(
        matches!(payload.data, PayloadData::Index(_)),
        "Expected IndexPayload"
    );

    Ok(())
}

#[test]
fn test_02_index_update_wormhole_bad_dest() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());

    let program = client.program(portal::ID)?;

    // Send index update to unsupported chain
    let err = program
        .request()
        .accounts(accounts::SendIndex {
            sender: program.payer(),
            system_program: system_program::ID,
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            bridge_adapter: wormhole_adapter::ID,
        })
        .args(instruction::SendIndex {
            destination_chain_id: 5,
        })
        .accounts(WormholeRemainingAccounts::account_metas(false))
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("Error Code: InvalidPeer"));

    Ok(())
}

#[test]
fn test_03_index_update_hyperlane() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let rpc_client = get_rpc_client();
    let program = client.program(portal::ID)?;

    // Fetch global
    let data_hyp = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID))?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;

    let accounts = HyperlaneRemainingAccounts::new(&program.payer(), &global_hp, None, false);

    // Send index update
    program
        .request()
        .accounts(accounts::SendIndex {
            sender: program.payer(),
            system_program: system_program::ID,
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            bridge_adapter: hyperlane_adapter::ID,
        })
        .args(instruction::SendIndex {
            destination_chain_id: 1,
        })
        .accounts(accounts.to_account_metas())
        .instruction(ComputeBudgetInstruction::set_compute_unit_limit(500_000))
        .send()?;

    let message_account = accounts.dispatched_message;
    let account_data = rpc_client.get_account_data(&message_account)?;

    let (payload, recipient) = util::hyperlane::decode_payload_from_message_account(&account_data)
        .expect("Failed to decode index payload");

    // Index should match the program’s current m_index (you send current index)
    let global_bytes = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &portal::ID))?;
    let portal_global = PortalGlobal::try_deserialize(&mut global_bytes.as_slice())?;
    assert_eq!(payload.header.index, portal_global.m_index);
    assert!(
        matches!(payload.data, PayloadData::Index(_)),
        "Expected IndexPayload"
    );

    // Recipient should be registered peer
    assert_eq!(
        hex::encode(recipient),
        "00000000000000000000000077ef4e9d37524069f81890c537a5c5d390bb4b4d"
    );

    Ok(())
}

#[test]
fn test_04_index_update_hyperlane_repeat_msgid() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let rpc_client = get_rpc_client();
    let program = client.program(portal::ID)?;

    // Fetch global
    let data_hyp = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID))?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;

    // user_global already exists, so message ID will be repeated
    let accounts = HyperlaneRemainingAccounts::new(&program.payer(), &global_hp, None, false);

    // Send index update
    let err = program
        .request()
        .accounts(accounts::SendIndex {
            sender: program.payer(),
            system_program: system_program::ID,
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            bridge_adapter: hyperlane_adapter::ID,
        })
        .args(instruction::SendIndex {
            destination_chain_id: 1,
        })
        .accounts(accounts.to_account_metas())
        .instruction(ComputeBudgetInstruction::set_compute_unit_limit(500_000))
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(
        s.contains("2006") || s.contains("custom program error: 0x07d6"),
        "unexpected error: {}",
        s
    );
    assert!(s.contains("ConstraintSeeds"));

    Ok(())
}

#[test]
fn test_05_index_update_hyperlane_bad_dest() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let rpc_client = get_rpc_client();
    let program = client.program(portal::ID)?;

    // Fetch global
    let data_hyp = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID))?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;
    let user_data_hyp = rpc_client.get_account_data(&pda!(
        &[GLOBAL_SEED, DASH_SEED, program.payer().as_ref()],
        &hyperlane_adapter::ID
    ))?;
    let user_hp = HyperlaneUserGlobal::try_deserialize(&mut user_data_hyp.as_slice())?;

    let accounts =
        HyperlaneRemainingAccounts::new(&program.payer(), &global_hp, Some(&user_hp), false);

    // Send index update
    let err = program
        .request()
        .accounts(accounts::SendIndex {
            sender: program.payer(),
            system_program: system_program::ID,
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            bridge_adapter: hyperlane_adapter::ID,
        })
        .args(instruction::SendIndex {
            destination_chain_id: 5,
        })
        .accounts(accounts.to_account_metas())
        .instruction(ComputeBudgetInstruction::set_compute_unit_limit(500_000))
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("Error Code: InvalidPeer"));

    Ok(())
}

#[test]
fn test_06_missing_account() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let program = client.program(portal::ID)?;

    let err = program
        .request()
        .accounts(accounts::SendIndex {
            sender: program.payer(),
            system_program: system_program::ID,
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            bridge_adapter: wormhole_adapter::ID,
        })
        .args(instruction::SendIndex {
            destination_chain_id: 1,
        })
        .accounts(WormholeRemainingAccounts::account_metas(false)[1..].to_vec())
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("Error Code: InvalidRemainingAccounts"), "{}", s);

    Ok(())
}

#[test]
fn test_07_send_merkle_root() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());

    let program = client.program(portal::ID)?;

    program
        .request()
        .accounts(accounts::SendMerkleRoot {
            sender: program.payer(),
            system_program: system_program::ID,
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            bridge_adapter: wormhole_adapter::ID,
            earn_global: pda!(&[GLOBAL_SEED], &earn::ID),
        })
        .args(instruction::SendMerkleRoot {
            destination_chain_id: 1,
        })
        .accounts(WormholeRemainingAccounts::account_metas(false))
        .send()?;

    Ok(())
}
