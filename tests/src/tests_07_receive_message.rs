use anchor_client::{Client, Cluster};
use anchor_lang::{prelude::AccountMeta, system_program};
use anyhow::Result;
use common::{
    pda, portal::constants::GLOBAL_SEED, require_metas,
    wormhole_adapter::constants::GUARDIAN_SET_SEED, wormhole_verify_vaa_shim, Payload,
    RegistrarListPayload, AUTHORITY_SEED,
};
use portal::state::MESSAGE_SEED;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use solana_transaction_status_client_types::UiTransactionEncoding;
use wormhole_adapter::{
    accounts, consts::CORE_BRIDGE_PROGRAM_ID, instruction, instructions::VaaBody,
};

use crate::{get_rpc_client, get_signer};

const SOLANA_EARNERS_LIST: [u8; 32] = [
    0x73, 0x6f, 0x6c, 0x61, 0x6e, 0x61, 0x2d, 0x65, 0x61, 0x72, 0x6e, 0x65, 0x72, 0x73, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[test]
fn test_01_add_registrar_earner() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [42u8; 32];
    let payload = Payload::RegistrarList(RegistrarListPayload {
        list_name: SOLANA_EARNERS_LIST,
        address: [1; 32],
        add: true,
        message_id,
    });

    let vaa = VaaBody {
        timestamp: 0,
        nonce: 0,
        emitter_chain: 2, // Ethereum
        emitter_address: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 99, 25, 106, 9, 21, 117, 173, 249, 158, 35, 6,
            229, 233, 14, 11, 229, 21, 72, 65, // Wormhole Transceiver on Ethereum
        ],
        sequence: 0,
        consistency_level: 0,
        payload: payload.clone(),
    };

    let metas = require_metas(
        &payload,
        signer.pubkey(),
        None,
        Some(Pubkey::from_str_const(
            "mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH",
        )),
        None,
    )?;

    // Relay message
    // Bridge validation is skipped with skip-validation feature flag
    let result = program
        .request()
        .accounts(accounts::ReceiveMessage {
            relayer: signer.pubkey(),
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
        })
        .args(instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send();

    // Expect to fail on Earn CPI until it is updated
    assert!(result.is_err());
    assert!(format!("{:?}", result.err().unwrap()).contains(
        "\"Program mz2vDzjbQDUDXBH6FPF5s4odCJ4y8YLE5QWaZ8XdZ9Z invoke [3]\", \"Program log: Instruction: AddRegistrarEarner\", \"Program log: AnchorError occurred. Error Code: InstructionDidNotDeserialize."
    ));

    Ok(())
}

#[test]
fn test_02_add_wrong_registrar_earner() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [42u8; 32];
    let payload = Payload::RegistrarList(RegistrarListPayload {
        list_name: SOLANA_EARNERS_LIST,
        address: [2; 32],
        add: true,
        message_id,
    });

    let vaa = VaaBody {
        timestamp: 0,
        nonce: 0,
        emitter_chain: 2, // Ethereum
        emitter_address: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 99, 25, 106, 9, 21, 117, 173, 249, 158, 35, 6,
            229, 233, 14, 11, 229, 21, 72, 65, // Wormhole Transceiver on Ethereum
        ],
        sequence: 0,
        consistency_level: 0,
        payload: payload.clone(),
    };

    let mut metas = require_metas(
        &payload,
        signer.pubkey(),
        None,
        Some(Pubkey::from_str_const(
            "mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH",
        )),
        None,
    )?;

    // Malicious relayer trying to add someone else as earner
    metas[2] = AccountMeta::new_readonly(Pubkey::new_from_array([3; 32]), false);

    // Relay message
    // Bridge validation is skipped with skip-validation feature flag
    let result = program
        .request()
        .accounts(accounts::ReceiveMessage {
            relayer: signer.pubkey(),
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
        })
        .args(instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send();

    // Fail validation in packages/common/src/accounts.rs
    assert!(result.is_err());
    assert!(format!("{:?}", result.err().unwrap())
        .contains("Error Message: Remaining account invalid."));

    Ok(())
}

#[test]
fn test_03_registrar_list_not_supported() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    // Unsupported list name
    let mut list_name = [0u8; 32];
    let s_bytes = b"fake-list";
    list_name[..s_bytes.len()].copy_from_slice(s_bytes);

    let message_id = [42u8; 32];
    let payload = Payload::RegistrarList(RegistrarListPayload {
        list_name,
        address: [2; 32],
        add: true,
        message_id,
    });

    let vaa = VaaBody {
        timestamp: 0,
        nonce: 0,
        emitter_chain: 2, // Ethereum
        emitter_address: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 99, 25, 106, 9, 21, 117, 173, 249, 158, 35, 6,
            229, 233, 14, 11, 229, 21, 72, 65, // Wormhole Transceiver on Ethereum
        ],
        sequence: 0,
        consistency_level: 0,
        payload: payload.clone(),
    };

    let metas = require_metas(
        &payload,
        signer.pubkey(),
        None,
        Some(Pubkey::from_str_const(
            "mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH",
        )),
        None,
    )?;

    // Relay message
    let result = program
        .request()
        .accounts(accounts::ReceiveMessage {
            relayer: signer.pubkey(),
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
        })
        .args(instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send()?;

    let transaction = get_rpc_client().get_transaction(&result, UiTransactionEncoding::Json)?;

    let logs = transaction
        .transaction
        .meta
        .as_ref()
        .unwrap()
        .log_messages
        .as_ref()
        .unwrap()
        .iter()
        .find(|log| {
            log.contains("Program log: Ignoring unsupported registrar list type: fake-list")
        });

    assert!(logs.is_some());

    Ok(())
}
