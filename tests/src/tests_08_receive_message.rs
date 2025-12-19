use anchor_client::{Client, Cluster};
use anchor_lang::{pubkey, system_program, InstructionData, ToAccountMetas};
use anyhow::Result;
use common::{
    hyperlane_adapter::constants::PAYER_SEED, pda, portal::constants::GLOBAL_SEED, require_metas,
    wormhole_adapter::constants::GUARDIAN_SET_SEED, wormhole_verify_vaa_shim, Payload,
    AUTHORITY_SEED,
};
use common::{IndexPayload, PayloadData, PayloadHeader};
use hyperlane_adapter::{accounts as hyperlane_accounts, instruction as hyperlane_instruction};
use portal::state::MESSAGE_SEED;
use solana_sdk::transaction::Transaction;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signer::Signer};
use wormhole_adapter::{
    accounts as wormhole_accounts, consts::CORE_BRIDGE_PROGRAM_ID,
    instruction as wormhole_instruction, instructions::VaaBody,
};

use crate::util::constants::{ETHEREUM_WORMHOLE_TRANSCEIVER, M_MINT, SOLANA_CHAIN_ID};
use crate::{get_rpc_client, get_signer};

#[test]
fn test_01_receive_index_wormhole() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [42u8; 32];
    let payload = create_default_payload(message_id, SOLANA_CHAIN_ID, portal::ID.to_bytes());
    let vaa = create_default_vaa(2, ETHEREUM_WORMHOLE_TRANSCEIVER, payload.clone());
    let metas = require_metas(&payload.data, signer.pubkey(), None, Some(M_MINT), None)?;

    // Relay message - Bridge validation is skipped with skip-validation feature flag
    let result = program
        .request()
        .accounts(create_receive_message_accounts(signer.pubkey(), message_id))
        .args(wormhole_instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send();

    // Expect to fail on Earn CPI until its portal authority is updated
    assert!(result.is_err());
    assert!(format!("{:?}", result.err().unwrap()).contains(
        "\"Program mz2vDzjbQDUDXBH6FPF5s4odCJ4y8YLE5QWaZ8XdZ9Z invoke [3]\", \"Program log: Instruction: PropagateIndex\", \"Program log: AnchorError caused by account: signer. Error Code: NotAuthorized. Error Number: 6002. Error Message: Invalid signer."
    ));

    Ok(())
}

#[test]
fn test_02_receive_index_hyperlane() -> Result<()> {
    let signer = get_signer();

    let message_id = [43u8; 32];
    let payload = Payload {
        header: PayloadHeader {
            message_id,
            payload_type: 1,
            destination_chain_id: 1399811149,
            destination_peer: portal::ID.to_bytes(),
        },
        data: PayloadData::Index(IndexPayload { index: 0 }),
    };

    let mut accounts = hyperlane_accounts::ReceiveMessage {
        receive_payer: pda!(&[PAYER_SEED], &hyperlane_adapter::ID),
        portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
        portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
        message_account: pda!(&[MESSAGE_SEED, &[43u8; 32]], &portal::ID),
        portal_program: portal::ID,
        system_program: system_program::ID,
        hyperlane_process_authority: Pubkey::default(),
        hyperlane_adapter_authority: pda!(&[AUTHORITY_SEED], &hyperlane_adapter::ID),
        hyperlane_global: pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID),
    }
    .to_account_metas(None);

    let metas = require_metas(&payload.data, signer.pubkey(), None, Some(M_MINT), None)?;
    accounts.extend_from_slice(&metas);

    // Remove hyperlane's mailbox process authority
    // Not needed with the skip-validation feature flag
    accounts = accounts[1..].to_vec();

    let instruction = Instruction {
        program_id: hyperlane_adapter::ID,
        accounts,
        data: hyperlane_instruction::ReceiveMessage {
            origin: 1,
            sender: [
                11, 106, 134, 128, 106, 3, 84, 200, 43, 143, 4, 158, 183, 93, 156, 151, 227, 112,
                166, 240, 192, 207, 161, 95, 71, 144, 156, 63, 225, 200, 247, 148,
            ],
            message: payload.encode(),
        }
        .data(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&signer.pubkey()),
        &[signer],
        get_rpc_client().get_latest_blockhash()?,
    );

    let result = get_rpc_client().send_and_confirm_transaction(&transaction);

    // Expect to fail on Earn CPI until its portal authority is updated
    assert!(result.is_err());
    assert!(format!("{:?}", result.err().unwrap()).contains(
        "\"Program mz2vDzjbQDUDXBH6FPF5s4odCJ4y8YLE5QWaZ8XdZ9Z invoke [3]\", \"Program log: Instruction: PropagateIndex\", \"Program log: AnchorError caused by account: signer. Error Code: NotAuthorized. Error Number: 6002. Error Message: Invalid signer."
    ));

    Ok(())
}

#[test]
fn test_03_receive_invalid_peer_address() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [42u8; 32];
    let payload = create_default_payload(message_id, SOLANA_CHAIN_ID, portal::ID.to_bytes());
    let vaa = create_default_vaa(2, [4; 32], payload.clone()); // Invalid emitter address
    let metas = require_metas(&payload.data, signer.pubkey(), None, Some(M_MINT), None)?;

    let result = program
        .request()
        .accounts(create_receive_message_accounts(signer.pubkey(), message_id))
        .args(wormhole_instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send();

    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Error Code: InvalidPeer"),
        "Invalid error: {}",
        err
    );

    Ok(())
}

#[test]
fn test_04_receive_invalid_peer_chain() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [42u8; 32];
    let payload = create_default_payload(message_id, SOLANA_CHAIN_ID, portal::ID.to_bytes());
    let vaa = create_default_vaa(2334, ETHEREUM_WORMHOLE_TRANSCEIVER, payload.clone()); // Invalid chain
    let metas = require_metas(&payload.data, signer.pubkey(), None, Some(M_MINT), None)?;

    let result = program
        .request()
        .accounts(create_receive_message_accounts(signer.pubkey(), message_id))
        .args(wormhole_instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send();

    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Error Code: InvalidPeer"),
        "Invalid error: {}",
        err
    );

    Ok(())
}

#[test]
fn test_05_receive_invalid_destination_chain() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [42u8; 32];
    let payload = create_default_payload(message_id, SOLANA_CHAIN_ID + 1, portal::ID.to_bytes()); // Invalid chain ID
    let vaa = create_default_vaa(2334, ETHEREUM_WORMHOLE_TRANSCEIVER, payload.clone());
    let metas = require_metas(&payload.data, signer.pubkey(), None, Some(M_MINT), None)?;

    let result = program
        .request()
        .accounts(create_receive_message_accounts(signer.pubkey(), message_id))
        .args(wormhole_instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send();

    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Error Code: InvalidPeer"),
        "Invalid error: {}",
        err
    );

    Ok(())
}

#[test]
fn test_06_receive_invalid_destination_peer() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [42u8; 32];
    let payload = create_default_payload(message_id, SOLANA_CHAIN_ID, [2; 32]); // Invalid peer
    let vaa = create_default_vaa(2334, ETHEREUM_WORMHOLE_TRANSCEIVER, payload.clone());
    let metas = require_metas(&payload.data, signer.pubkey(), None, Some(M_MINT), None)?;

    let result = program
        .request()
        .accounts(create_receive_message_accounts(signer.pubkey(), message_id))
        .args(wormhole_instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send();

    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Error Code: InvalidPeer"),
        "Invalid error: {}",
        err
    );

    Ok(())
}

fn create_default_payload(
    message_id: [u8; 32],
    destination_chain_id: u32,
    destination_peer: [u8; 32],
) -> Payload {
    Payload {
        header: PayloadHeader {
            message_id,
            payload_type: 1,
            destination_chain_id,
            destination_peer,
        },
        data: PayloadData::Index(IndexPayload { index: 0 }),
    }
}

fn create_default_vaa(emitter_chain: u16, emitter_address: [u8; 32], payload: Payload) -> VaaBody {
    VaaBody {
        timestamp: 0,
        nonce: 0,
        emitter_chain,
        emitter_address,
        sequence: 0,
        consistency_level: 0,
        payload,
    }
}

fn create_receive_message_accounts(
    signer_pubkey: Pubkey,
    message_id: [u8; 32],
) -> wormhole_accounts::ReceiveMessage {
    wormhole_accounts::ReceiveMessage {
        relayer: signer_pubkey,
        wormhole_global: pda!(&[GLOBAL_SEED], &wormhole_adapter::ID),
        portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
        wormhole_adapter_authority: pda!(&[AUTHORITY_SEED], &wormhole_adapter::ID),
        portal_authority: pda!(&[AUTHORITY_SEED], &portal::ID),
        message_account: pda!(&[MESSAGE_SEED, &message_id], &portal::ID),
        guardian_set: pda!(
            &[GUARDIAN_SET_SEED, &0u32.to_be_bytes()],
            &CORE_BRIDGE_PROGRAM_ID
        ),
        guardian_signatures: Pubkey::default(),
        wormhole_verify_vaa_shim: wormhole_verify_vaa_shim::ID,
        portal_program: portal::ID,
        system_program: system_program::ID,
    }
}
