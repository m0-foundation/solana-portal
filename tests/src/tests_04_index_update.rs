use anchor_client::{Client, Cluster};
use anchor_lang::system_program;
use anyhow::Result;
use common::{
    hyperlane_adapter, pda,
    portal::constants::GLOBAL_SEED,
    wormhole_adapter::{self, constants::EMITTER_SEED},
    HyperlaneRemainingAccounts, Payload, WormholeRemainingAccounts, AUTHORITY_SEED,
};
use portal::{accounts, instruction};
use solana_sdk::bs58;
use solana_transaction_status_client_types::{UiInstruction, UiTransactionEncoding};

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
            messenger_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            bridge_adapter: wormhole_adapter::ID,
        })
        .args(instruction::SendIndex {
            destination_chain_id: 2,
        })
        .accounts(WormholeRemainingAccounts::account_metas())
        .send()?;

    let transaction = rpc_client.get_transaction(&signature, UiTransactionEncoding::Json)?;

    // Find and verify the wormhole post message event
    let meta = transaction
        .transaction
        .meta
        .as_ref()
        .expect("Transaction meta missing");

    let inner_instructions = meta
        .inner_instructions
        .as_ref()
        .expect("Inner instructions missing");

    let found_event = inner_instructions
        .iter()
        .flat_map(|inner| &inner.instructions)
        .find_map(|ix| {
            let UiInstruction::Compiled(compiled_ix) = ix else {
                return None;
            };

            let data = bs58::decode(&compiled_ix.data).into_vec().ok()?;

            // Verify Event CPI discriminator and extract data
            if data.get(0..8)? != [228, 69, 165, 46, 81, 203, 154, 29] {
                return None;
            }

            let emitter = data.get(16..48)?;
            let sequence = u64::from_le_bytes(data.get(48..56)?.try_into().ok()?);
            let timestamp = u32::from_le_bytes(data.get(56..60)?.try_into().ok()?);

            // Verify all conditions
            let expected_emitter = pda!(&[EMITTER_SEED], &wormhole_adapter::ID).to_bytes();
            if emitter == expected_emitter && sequence > 50 && timestamp > 0 {
                return Some(());
            }

            None
        });

    assert!(
        found_event.is_some(),
        "Wormhole post message event not found or invalid"
    );

    let message_account = WormholeRemainingAccounts::new().message_account;
    let account_data = rpc_client.get_account_data(&message_account)?;

    // Emitter chain and address
    assert_eq!(account_data[57..59], [1, 0]);
    assert_eq!(
        account_data[59..91],
        WormholeRemainingAccounts::new().emitter.to_bytes()
    );

    Ok(())
}

#[test]
fn test_02_index_update_hyperlane() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
    let rpc_client = get_rpc_client();
    let program = client.program(portal::ID)?;

    // Send index update
    program
        .request()
        .accounts(accounts::SendIndex {
            sender: program.payer(),
            system_program: system_program::ID,
            portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
            messenger_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
            bridge_adapter: hyperlane_adapter::ID,
        })
        .args(instruction::SendIndex {
            destination_chain_id: 1,
        })
        .accounts(HyperlaneRemainingAccounts::account_metas(0))
        .send()?;

    let message_account = HyperlaneRemainingAccounts::new(0).dispatched_message;
    let account_data = rpc_client.get_account_data(&message_account)?;

    // The last 40 bytes of the account data contain the message body
    let len = account_data.len();
    let message_body = &account_data[len - 41..];
    let message = Payload::decode(&message_body.to_vec());

    let Payload::Index(index) = message else {
        panic!("Expected IndexPayload");
    };

    // Default index is 0
    assert_eq!(index.index, 0);

    // Recipient should be registered peer
    let recipient = &account_data[len - 73..len - 41];
    assert_eq!(
        hex::encode(recipient),
        "000000000000000000000000d925c84b55e4e44a53749ff5f2a5a13f63d128fd"
    );

    Ok(())
}
