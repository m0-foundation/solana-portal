use anchor_client::{Client, Cluster};
use anchor_lang::{system_program, AccountDeserialize};
use anyhow::Result;
use common::{
    hyperlane_adapter::{self, accounts::HyperlaneGlobal},
    pda,
    portal::constants::GLOBAL_SEED,
    wormhole_adapter::{self, constants::EMITTER_SEED},
    HyperlaneRemainingAccounts, WormholeRemainingAccounts, AUTHORITY_SEED,
};
use portal::{
    state::PortalGlobal,
    {accounts, instruction},
};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_transaction_status_client_types::UiTransactionEncoding;

use crate::{get_rpc_client, get_signer};

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
            destination_chain_id: 2,
        })
        .accounts(WormholeRemainingAccounts::account_metas())
        .send()?;

    let transaction = rpc_client.get_transaction(&signature, UiTransactionEncoding::Json)?;

    let event_meta = crate::util::wormhole::find_message_event_in_tx(&transaction)
    .expect("Wormhole post message event not found or invalid");

    let expected_emitter = pda!(&[EMITTER_SEED], &wormhole_adapter::ID).to_bytes();
    assert_eq!(event_meta.emitter, expected_emitter);
    assert!(event_meta.sequence > 50);
    assert!(event_meta.timestamp > 0);

    let message_account = WormholeRemainingAccounts::new().message_account;
    let account_data = rpc_client.get_account_data(&message_account)?;

    // Emitter chain and address
    assert_eq!(account_data[57..59], [1, 0]);
    assert_eq!(
        account_data[59..91],
        WormholeRemainingAccounts::new().emitter.to_bytes()
    );

    let index_payload = crate::util::wormhole::find_index_payload_in_tx(&transaction)
    .expect("Index payload not found");

    // Index should match the program’s current m_index (you send current index)
    let global_bytes = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &portal::ID))?;
    let portal_global = PortalGlobal::try_deserialize(&mut global_bytes.as_slice())?;

    assert_eq!(index_payload.index, portal_global.m_index);

    // Message ID should match expected computation
    let expected_message_id = crate::util::compute_expected_message_id(
        portal_global.chain_id,
        portal_global.message_nonce,
    );
    assert_eq!(index_payload.message_id, expected_message_id);

    Ok(())
}

#[test]
fn test_02_index_update_wormhole_wrong_dest() -> Result<()> {
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
        .accounts(WormholeRemainingAccounts::account_metas())
        .send()
        .unwrap_err();

    let s = err.to_string();
    assert!(s.contains("Custom(6008)") || s.contains("custom program error: 0x1778"));
    assert!(s.contains("UnsupportedDestinationChain"));

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

    let accounts = HyperlaneRemainingAccounts::new(&program.payer(), &global_hp, None);

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

    let payload = crate::util::hyperlane::decode_message_account_index_payload(&account_data)
        .expect("Failed to decode index payload");

    // Index should match the program’s current m_index (you send current index)
    let global_bytes = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &portal::ID))?;
    let portal_global = PortalGlobal::try_deserialize(&mut global_bytes.as_slice())?;

    // Default index is 0
    assert_eq!(payload.index, portal_global.m_index);

    // Message ID should match expected computation
    let expected_message_id = crate::util::compute_expected_message_id(
        portal_global.chain_id,
        portal_global.message_nonce,
    );
    assert_eq!(payload.message_id, expected_message_id);

    // Recipient should be registered peer
    let len = account_data.len();
    let recipient = &account_data[len - 73..len - 41];
    assert_eq!(
        hex::encode(recipient),
        "0b6a86806a0354c82b8f049eb75d9c97e370a6f0c0cfa15f47909c3fe1c8f794"
    );

    Ok(())
}

#[test]
fn test_02_index_update_hyperlane_wrong_dest() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let rpc_client = get_rpc_client();

    let program = client.program(portal::ID)?;

    // Fetch global
    let data_hyp = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID))?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;

    let accounts = HyperlaneRemainingAccounts::new(&program.payer(), &global_hp, None);

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
    assert!(s.contains("Custom(6008)") || s.contains("custom program error: 0x1778"));
    assert!(s.contains("UnsupportedDestinationChain"));

    Ok(())
}
