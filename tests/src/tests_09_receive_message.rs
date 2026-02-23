use std::str::FromStr;

use anchor_client::{Client, Cluster};
use anchor_lang::{system_program, AnchorDeserialize, InstructionData, ToAccountMetas};
use anchor_spl::token_2022;
use anyhow::Result;
use hyperlane_adapter::{accounts as hyperlane_accounts, instruction as hyperlane_instruction};
use m0_portal_common::ext_swap::accounts::SwapGlobal;
use m0_portal_common::portal::constants::PORTAL_AUTHORITY_SEED;
use m0_portal_common::{
    earn, ext_swap, CancelReportPayload, EarnerMerkleRootPayload, Extension, IndexPayload,
    PayloadData, PayloadHeader, TokenTransferPayload,
};
use m0_portal_common::{
    hyperlane_adapter::constants::PAYER_SEED, pda, portal::constants::GLOBAL_SEED, require_metas,
    wormhole_adapter::constants::GUARDIAN_SET_SEED, wormhole_verify_vaa_shim, Payload,
    AUTHORITY_SEED,
};
use portal::state::MESSAGE_SEED;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::transaction::Transaction;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signer::Signer};
use solana_transaction_status_client_types::UiTransactionEncoding;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use wormhole_adapter::{
    accounts as wormhole_accounts, consts::CORE_BRIDGE_PROGRAM_ID,
    instruction as wormhole_instruction, instructions::VaaBody,
};

use crate::util::constants::{
    ETHEREUM_HYPERLANE_ADAPTER, ETHEREUM_WORMHOLE_ADAPTER, M_MINT, SOLANA_CHAIN_ID,
};
use crate::util::wormhole::build_versioned_tx_with_lut;
use crate::{get_rpc_client, get_signer};

#[test]
fn test_01_receive_index_wormhole() -> Result<()> {
    let rpc_client = get_rpc_client();
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [42u8; 32];
    let payload = create_default_payload(message_id, SOLANA_CHAIN_ID, portal::ID.to_bytes());
    let vaa = create_default_vaa(2, ETHEREUM_WORMHOLE_ADAPTER, payload.clone());
    let metas = require_metas(&payload.data, M_MINT, vec![], None)?;

    // Relay message - Bridge validation is skipped with skip-validation feature flag
    let signature = program
        .request()
        .accounts(create_receive_message_accounts(signer.pubkey(), message_id))
        .args(wormhole_instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send()?;

    // Verify that the index was propagated
    let transaction = rpc_client.get_transaction(&signature, UiTransactionEncoding::Json)?;
    let logs = transaction
        .transaction
        .meta
        .unwrap()
        .log_messages
        .unwrap()
        .join(". ");

    assert!(
        logs.contains("Program mz2vDzjbQDUDXBH6FPF5s4odCJ4y8YLE5QWaZ8XdZ9Z invoke [3]. Program log: Instruction: PropagateIndex."),
        "Missing PropagateIndex log: {:?}",
        logs
    );

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
            index: 0,
        },
        data: PayloadData::Index(IndexPayload {}),
    };

    let mut accounts = hyperlane_accounts::ReceiveMessage {
        receive_payer: pda!(&[PAYER_SEED], &hyperlane_adapter::ID),
        portal_global: pda!(&[GLOBAL_SEED], &portal::ID),
        portal_authority: pda!(&[PORTAL_AUTHORITY_SEED], &portal::ID),
        message_account: pda!(&[MESSAGE_SEED, &[43u8; 32]], &portal::ID),
        portal_program: portal::ID,
        system_program: system_program::ID,
        hyperlane_process_authority: Pubkey::default(),
        hyperlane_adapter_authority: pda!(&[AUTHORITY_SEED], &hyperlane_adapter::ID),
        hyperlane_global: pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID),
        earn_global: pda!(&[GLOBAL_SEED], &earn::ID),
        m_mint: M_MINT,
        m_token_program: token_2022::ID,
        earn_program: earn::ID,
    }
    .to_account_metas(None);

    let metas = require_metas(&payload.data, M_MINT, vec![], None)?;
    accounts.extend_from_slice(&metas);

    // Remove hyperlane's mailbox process authority
    // Not needed with the skip-validation feature flag
    accounts = accounts[1..].to_vec();

    let instruction = Instruction {
        program_id: hyperlane_adapter::ID,
        accounts,
        data: hyperlane_instruction::ReceiveMessage {
            origin: 1,
            sender: ETHEREUM_HYPERLANE_ADAPTER,
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

    let client = get_rpc_client();
    let signature = client.send_and_confirm_transaction(&transaction)?;

    // Verify that the index was propagated
    let transaction = client.get_transaction(&signature, UiTransactionEncoding::Json)?;
    let logs = transaction
        .transaction
        .meta
        .unwrap()
        .log_messages
        .unwrap()
        .join(". ");

    assert!(
        logs.contains("Program mz2vDzjbQDUDXBH6FPF5s4odCJ4y8YLE5QWaZ8XdZ9Z invoke [3]. Program log: Instruction: PropagateIndex."),
        "Missing PropagateIndex log: {:?}",
        logs
    );
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
    let metas = require_metas(&payload.data, M_MINT, vec![], None)?;

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
    let vaa = create_default_vaa(2334, ETHEREUM_WORMHOLE_ADAPTER, payload.clone()); // Invalid chain
    let metas = require_metas(&payload.data, M_MINT, vec![], None)?;

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
    let vaa = create_default_vaa(2334, ETHEREUM_WORMHOLE_ADAPTER, payload.clone());
    let metas = require_metas(&payload.data, M_MINT, vec![], None)?;

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
    let vaa = create_default_vaa(2334, ETHEREUM_WORMHOLE_ADAPTER, payload.clone());
    let metas = require_metas(&payload.data, M_MINT, vec![], None)?;

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
fn test_07_receive_cancel_wormhole() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [44u8; 32];
    let mut payload = create_default_payload(message_id, SOLANA_CHAIN_ID, portal::ID.to_bytes());

    payload.header.payload_type = 6; // CancelReport
    payload.data = PayloadData::CancelReport(CancelReportPayload {
        order_id: [0; 32],
        order_sender: [0; 32],
        token_in: [0; 32],
        amount_in_to_refund: 10,
    });

    let vaa = create_default_vaa(2, ETHEREUM_WORMHOLE_ADAPTER, payload.clone());
    let metas = require_metas(&payload.data, M_MINT, vec![], None)?;

    let result = program
        .request()
        .accounts(create_receive_message_accounts(signer.pubkey(), message_id))
        .args(wormhole_instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send();

    // the order is fake
    // check for order book error
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Instruction: ReportOrderCancel"),
        "Invalid error: {}",
        err
    );
    assert!(
        err.contains("AnchorError caused by account: order"),
        "Invalid error: {}",
        err
    );

    Ok(())
}

#[test]
fn test_08_change_destination_mint() -> Result<()> {
    let rpc_client = get_rpc_client();
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [45u8; 32];
    let mut payload = create_default_payload(message_id, SOLANA_CHAIN_ID, portal::ID.to_bytes());

    payload.header.payload_type = 0;
    payload.data = PayloadData::TokenTransfer(TokenTransferPayload {
        recipient: signer.pubkey().to_bytes(),
        destination_token: Pubkey::new_unique().to_bytes(),
        amount: 1_000_000,
        sender: get_signer().pubkey().to_bytes(),
    });

    let data_swap = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &ext_swap::ID))?;
    let swap_global = SwapGlobal::deserialize(&mut &data_swap[8..])?;
    let whitelisted_extensions = swap_global
        .whitelisted_extensions
        .iter()
        .map(|&ext| Extension::from(ext))
        .collect();

    let vaa = create_default_vaa(2, ETHEREUM_WORMHOLE_ADAPTER, payload.clone());
    let mut metas = require_metas(&payload.data, M_MINT, whitelisted_extensions, None)?;

    // Change destination mint to an different mint (should fail validation)
    metas[0].pubkey = Pubkey::from_str("usdkbee86pkLyRmxfFCdkyySpxRb5ndCxVsK2BkRXwX").unwrap();

    let result = program
        .request()
        .accounts(create_receive_message_accounts(signer.pubkey(), message_id))
        .args(wormhole_instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .send_with_spinner_and_config(RpcSendTransactionConfig {
            skip_preflight: true,
            ..Default::default()
        });

    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("custom program error: 0x1771"),
        "Invalid error: {}",
        err
    );

    Ok(())
}

#[test]
fn test_09_receive_merkle_root() -> Result<()> {
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [47u8; 32];

    let mut payload = create_default_payload(message_id, SOLANA_CHAIN_ID, portal::ID.to_bytes());
    payload.header.payload_type = 5;
    payload.data = PayloadData::EarnerMerkleRoot(EarnerMerkleRootPayload {
        merkle_root: [7u8; 32],
    });

    // Message coming from Arbitrum
    let vaa = create_default_vaa(23, ETHEREUM_WORMHOLE_ADAPTER, payload.clone());
    let metas = require_metas(&payload.data, M_MINT, vec![], None)?;

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
    assert!(err.contains("InvalidPeer"), "Invalid error: {}", err);

    Ok(())
}

#[test]
fn test_10_uninitialized_token_account() -> Result<()> {
    let rpc_client = get_rpc_client();
    let signer = get_signer();
    let client = Client::new(Cluster::Localnet, signer.clone());
    let program = client.program(wormhole_adapter::ID)?;

    let message_id = [48u8; 32];
    let mut payload = create_default_payload(message_id, SOLANA_CHAIN_ID, portal::ID.to_bytes());
    let mint = Pubkey::from_str("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp")?;
    let rand_recipient = Pubkey::new_unique();

    payload.header.payload_type = 0;
    payload.data = PayloadData::TokenTransfer(TokenTransferPayload {
        recipient: rand_recipient.to_bytes(),
        destination_token: mint.to_bytes(),
        amount: 1_000_000,
        sender: get_signer().pubkey().to_bytes(),
    });

    let data_swap = rpc_client.get_account_data(&pda!(&[GLOBAL_SEED], &ext_swap::ID))?;
    let swap_global = SwapGlobal::deserialize(&mut &data_swap[8..])?;
    let whitelisted_extensions = swap_global
        .whitelisted_extensions
        .iter()
        .map(|&ext| Extension::from(ext))
        .collect();

    let vaa = create_default_vaa(2, ETHEREUM_WORMHOLE_ADAPTER, payload.clone());
    let metas = require_metas(&payload.data, M_MINT, whitelisted_extensions, None)?;

    // ensure the account is not initialized
    let ata = get_associated_token_address_with_program_id(&rand_recipient, &mint, &token_2022::ID);
    let account = rpc_client.get_account(&ata);
    assert!(account.is_err());

    let instructions = program
        .request()
        .accounts(create_receive_message_accounts(signer.pubkey(), message_id))
        .args(wormhole_instruction::ReceiveMessage {
            vaa_body: vaa.to_bytes(),
            guardian_set_index: 0,
        })
        .accounts(metas)
        .instruction(ComputeBudgetInstruction::set_compute_unit_limit(600_000))
        .instructions()?;

    let versioned_tx = build_versioned_tx_with_lut(rpc_client.clone(), instructions)?;

    let result = rpc_client.send_and_confirm_transaction(&versioned_tx);
    assert!(result.is_ok(), "error: {:?}", result.err());

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
            index: 0,
        },
        data: PayloadData::Index(IndexPayload {}),
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
        portal_authority: pda!(&[PORTAL_AUTHORITY_SEED], &portal::ID),
        message_account: pda!(&[MESSAGE_SEED, &message_id], &portal::ID),
        guardian_set: pda!(
            &[GUARDIAN_SET_SEED, &0u32.to_be_bytes()],
            &CORE_BRIDGE_PROGRAM_ID
        ),
        guardian_signatures: Pubkey::default(),
        wormhole_verify_vaa_shim: wormhole_verify_vaa_shim::ID,
        portal_program: portal::ID,
        system_program: system_program::ID,
        earn_global: pda!(&[GLOBAL_SEED], &earn::ID),
        m_mint: M_MINT,
        m_token_program: token_2022::ID,
        earn_program: earn::ID,
    }
}
