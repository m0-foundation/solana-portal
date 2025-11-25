use anchor_client::{Client, Cluster};
use anchor_lang::system_program;
use anyhow::Result;
use common::{
    hyperlane_adapter, pda,
    portal::constants::GLOBAL_SEED,
    wormhole_adapter::{self, constants::EMITTER_SEED},
    wormhole_post_message_shim, HyperlaneRemainingAccounts, WormholeRemainingAccounts,
    AUTHORITY_SEED,
};
use portal::{accounts, instruction};
use solana_sdk::bs58;
use solana_transaction_status_client_types::{
    EncodedTransaction, UiInstruction, UiMessage, UiTransactionEncoding,
};

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

    // Extract account keys from transaction
    let account_keys = match transaction.transaction.transaction {
        EncodedTransaction::Json(ref t) => match &t.message {
            UiMessage::Raw(message) => &message.account_keys,
            _ => panic!("Expected raw message format"),
        },
        _ => panic!("Expected JSON encoded transaction"),
    };

    // Event CPI instruction discriminator
    const EVENT_DISCRIMINATOR: [u8; 8] = [228, 69, 165, 46, 81, 203, 154, 29];

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

            let program_id = &account_keys[compiled_ix.program_id_index as usize];
            if program_id != &wormhole_post_message_shim::ID.to_string() {
                return None;
            }

            let data = bs58::decode(&compiled_ix.data).into_vec().ok()?;

            // Verify discriminator and extract data
            let disc = data.get(0..8)?;
            if disc != EVENT_DISCRIMINATOR {
                return None;
            }

            let emitter = data.get(16..48)?;
            let sequence = data.get(48..56)?;
            let timestamp = u32::from_le_bytes(data.get(56..60)?.try_into().ok()?);

            // Verify all conditions
            let expected_emitter = pda!(&[EMITTER_SEED], &wormhole_adapter::ID).to_bytes();
            if emitter == expected_emitter && sequence == [0u8; 8] && timestamp > 0 {
                return Some(());
            }

            None
        });

    assert!(
        found_event.is_some(),
        "Wormhole post message event not found or invalid"
    );

    // READ message account data to verify contents

    Ok(())
}

#[test]
fn test_02_index_update_hyperlane() -> Result<()> {
    let client = Client::new(Cluster::Localnet, get_signer());
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
        .accounts(HyperlaneRemainingAccounts::account_metas())
        .send()?;

    Ok(())
}
