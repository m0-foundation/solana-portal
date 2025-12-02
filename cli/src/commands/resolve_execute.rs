use anchor_client::solana_account_decoder::{UiAccount, UiAccountData, UiAccountEncoding};
use anyhow::{Ok, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use borsh::BorshDeserialize;
use bs58;
use common::{
    pda,
    wormhole_adapter::{self, constants::GUARDIAN_SET_SEED},
};
use core::panic;
use executor_account_resolver_svm::{InstructionGroups, Resolver};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcSimulateTransactionAccountsConfig};
use solana_rpc_client_types::config::RpcSimulateTransactionConfig;
use solana_sdk::{
    address_lookup_table::{state::AddressLookupTable, AddressLookupTableAccount},
    commitment_config::{CommitmentConfig, CommitmentLevel},
    instruction::{AccountMeta, Instruction as SolanaInstruction},
    message::{v0, Message, VersionedMessage},
    packet::Encode,
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use solana_transaction_status_client_types::{
    EncodedTransaction, UiMessage, UiTransactionEncoding,
};
use std::{collections::HashSet, result::Result as StdResult, thread::sleep, time::Duration};

use crate::types::*;

const GUARDIAN_SET_INDEX_SEED: u32 = 0;
pub const CORE_BRIDGE_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("3u8hJUVTA4jH1wYAyUur7FFZVQ8H635K3tSHHF4ssjQ5");
const TARGET_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("EFaNWErqAtVWufdNb7yofSHHfWFos843DFpu4JBw24at");
pub const RESOLVER_PUBKEY_SHIM_VAA_SIGS: Pubkey =
    Pubkey::new_from_array(*b"shim_vaa_sigs_000000000000000000");
const CUSTOM_LUT: Option<Pubkey> = Some(Pubkey::from_str_const(
    "49zK4hBSgwoNCukBTtGVLkQEr7NUkCfebTb7dqM7uhmG",
));

pub fn resolve_execute(tx_hash: String) -> Result<()> {
    let rpc_client = RpcClient::new("https://lindsy-gxe51w-fast-devnet.helius-rpc.com");
    let key_path = format!("{}/.config/solana/id.json", std::env::var("HOME")?);
    let payer = Keypair::read_from_file(key_path).expect("failed to read keypair");

    // Make request to wormhole to get VAA
    let api_client = reqwest::blocking::Client::new();
    let url = format!(
        "https://api.testnet.wormholescan.io/api/v1/operations?txHash={}",
        tx_hash,
    );

    let response = api_client.get(url).send()?;
    let api_response: WormholeResponse = response.json()?;

    let response = api_response
        .operations
        .get(0)
        .expect("no executor operations returned");

    let vaa_bytes = BASE64_STANDARD.decode(&response.vaa.raw)?;

    // Extract VAA body by skipping the header
    let header_len = 6 + vaa_bytes[5] as usize * 66;
    let vaa_body = vaa_bytes[header_len..].to_vec();

    // Make POST request to executor API to get transaction status
    let executor_url = "https://executor-testnet.labsapis.com/v0/status/tx";
    let executor_body = serde_json::json!({
        "txHash": tx_hash
    });

    let executor_response = api_client.post(executor_url).json(&executor_body).send()?;
    let executor_txs: ExecutorTransactions = executor_response.json()?;

    let mut guardian_set: Option<Pubkey> = None;

    // Result PDA that holds the final result
    let result_pda = pda!(
        &[b"executor-account-resolver:result"],
        &wormhole_adapter::ID
    );

    // Get the transaction hash from the first item in txs
    let extracted_instruction = if let Some(executor_tx) = executor_txs.get(0) {
        if let Some(tx) = executor_tx.txs.get(0) {
            let tx_hash_sig = tx.tx_hash.parse()?;

            // Fetch the transaction using RPC client with JSON encoding
            let transaction = rpc_client.get_transaction_with_config(
                &tx_hash_sig,
                solana_rpc_client_types::config::RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    commitment: None,
                    max_supported_transaction_version: Some(0),
                },
            )?;

            // Extract the instruction to the target program
            let ui_transaction = transaction.transaction.transaction;
            if let EncodedTransaction::Json(ui_tx) = ui_transaction {
                if let UiMessage::Raw(raw_msg) = ui_tx.message {
                    // Find the instruction to the target program (skip compute budget instructions)
                    let mut target_instruction: Option<SolanaInstruction> = None;

                    for ix in raw_msg.instructions {
                        let program_id_index: usize = ix.program_id_index.into();
                        if program_id_index < raw_msg.account_keys.len() {
                            let program_id_str = &raw_msg.account_keys[program_id_index];
                            match program_id_str.parse::<Pubkey>() {
                                StdResult::Ok(program_id) => {
                                    if program_id == TARGET_PROGRAM_ID {
                                        // Reconstruct the instruction
                                        let num_required_signatures: usize =
                                            raw_msg.header.num_required_signatures.into();
                                        let num_readonly_signed: usize =
                                            raw_msg.header.num_readonly_signed_accounts.into();
                                        let num_readonly_unsigned: usize =
                                            raw_msg.header.num_readonly_unsigned_accounts.into();

                                        let accounts: Vec<AccountMeta> = ix
                                            .accounts
                                            .iter()
                                            .map(|&idx| {
                                                let idx_usize: usize = idx.into();
                                                let pubkey = raw_msg.account_keys[idx_usize]
                                                    .parse::<Pubkey>()
                                                    .unwrap();

                                                let is_signer = idx_usize < num_required_signatures;
                                                let is_writable = if is_signer {
                                                    idx_usize
                                                        < num_required_signatures
                                                            - num_readonly_signed
                                                } else {
                                                    idx_usize
                                                        < raw_msg.account_keys.len()
                                                            - num_readonly_unsigned
                                                };

                                                AccountMeta {
                                                    pubkey,
                                                    is_signer,
                                                    is_writable,
                                                }
                                            })
                                            .collect();

                                        guardian_set = Some(accounts[1].pubkey);

                                        target_instruction = Some(SolanaInstruction {
                                            program_id,
                                            accounts,
                                            data: bs58::decode(&ix.data)
                                                .into_vec()
                                                .expect("failed to decode instruction data"),
                                        });
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    target_instruction
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let mut requested_accounts = vec![];
    let mut result_data: Option<UiAccount>;

    // Simulate until we get final result
    loop {
        let transaction = create_transaction(
            vaa_body.clone(),
            &rpc_client,
            requested_accounts.clone(),
            payer.pubkey(),
        )?;

        // Simulate transaction
        let simulation_result = rpc_client.simulate_transaction_with_config(
            &transaction,
            RpcSimulateTransactionConfig {
                sig_verify: false,
                replace_recent_blockhash: true,
                accounts: Some(RpcSimulateTransactionAccountsConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    addresses: vec![result_pda.to_string()],
                }),
                ..RpcSimulateTransactionConfig::default()
            },
        )?;

        result_data = simulation_result
            .value
            .accounts
            .and_then(|accounts| accounts.into_iter().next().flatten());

        // Parse the return data
        let return_data = simulation_result.value.return_data.unwrap();
        let data_bytes = BASE64_STANDARD.decode(&return_data.data.0)?;

        // No more missing accounts
        if data_bytes[0] != 1 {
            break;
        }

        let accounts_len = u32::from_le_bytes(data_bytes[1..5].try_into()?) as usize;

        println!("=== Missing Accounts ===");
        let mut missing_accounts = vec![];

        let mut offset = 5;
        for j in 0..accounts_len {
            if offset + 32 <= data_bytes.len() {
                let pubkey_bytes = &data_bytes[offset..offset + 32];
                let pubkey = Pubkey::try_from(pubkey_bytes)?;
                println!("Account {}: {}", j, pubkey);
                missing_accounts.push(pubkey);
                offset += 32;
            }
        }

        // Add missing accounts for next iteration
        requested_accounts.extend(missing_accounts.into_iter().map(|pubkey| AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: pubkey == result_pda,
        }));
    }

    let data = result_data.expect("expected final result data").data;
    let decoded_data = match data {
        UiAccountData::Binary(encoded, _) => BASE64_STANDARD.decode(encoded)?,
        _ => anyhow::bail!("Unexpected account data encoding"),
    };

    // Skip the 8-byte discriminator and deserialize the Resolver directly
    let result = Resolver::<InstructionGroups>::deserialize(&mut &decoded_data[8..])?;

    println!("\n=== Resolver Result ===");

    match &result {
        Resolver::Resolved(instruction_groups) => {
            for group in instruction_groups.0.iter() {
                let mut instructions = vec![extracted_instruction
                    .clone()
                    .expect("missing post vaa instruction")];

                for serializable_ix in group.instructions.iter() {
                    // Parse instruction
                    let accounts: Vec<AccountMeta> = serializable_ix
                        .accounts
                        .iter()
                        .map(|acc| AccountMeta {
                            pubkey: if acc.pubkey == RESOLVER_PUBKEY_SHIM_VAA_SIGS {
                                guardian_set.expect("missing gaurdian set signatures account")
                            } else {
                                acc.pubkey
                            },
                            is_signer: acc.is_signer,
                            is_writable: acc.is_writable,
                        })
                        .collect();

                    let instruction = SolanaInstruction {
                        program_id: serializable_ix.program_id,
                        accounts,
                        data: serializable_ix.data.clone(),
                    };

                    instructions.push(instruction);
                }

                // Fetch LUTs
                let mut address_lookup_tables = Vec::new();
                for lut_pubkey in group.address_lookup_tables.iter() {
                    match rpc_client.get_account(lut_pubkey) {
                        StdResult::Ok(account) => {
                            match AddressLookupTable::deserialize(&account.data) {
                                StdResult::Ok(lut) => {
                                    let lut_account = AddressLookupTableAccount {
                                        key: *lut_pubkey,
                                        addresses: lut.addresses.to_vec(),
                                    };
                                    address_lookup_tables.push(lut_account);
                                }
                                StdResult::Err(e) => {
                                    println!(
                                        "\n  Warning: Failed to deserialize LUT {}: {}",
                                        lut_pubkey, e
                                    );
                                }
                            }
                        }
                        StdResult::Err(e) => {
                            println!("\n  Warning: Failed to fetch LUT {}: {}", lut_pubkey, e);
                        }
                    }
                }

                // Create new LUT with accounts from the transaction
                let custom_lut = if let Some(lut) = CUSTOM_LUT {
                    lut
                } else {
                    let mut all_accounts = HashSet::new();
                    for instruction in instructions.iter() {
                        for account in instruction.accounts.iter() {
                            all_accounts.insert(account.pubkey);
                        }
                    }

                    let lut_addresses: Vec<Pubkey> = all_accounts.into_iter().collect();
                    let recent_slot = rpc_client.get_slot_with_commitment(CommitmentConfig {
                        commitment: CommitmentLevel::Finalized,
                    })?;

                    let (create_lut_ix, lut_address) =
                        solana_sdk::address_lookup_table::instruction::create_lookup_table(
                            payer.pubkey(),
                            payer.pubkey(),
                            recent_slot - 50,
                        );

                    let extend_ix =
                        solana_sdk::address_lookup_table::instruction::extend_lookup_table(
                            lut_address,
                            payer.pubkey(),
                            Some(payer.pubkey()),
                            lut_addresses.clone(),
                        );

                    let lut_tx = Transaction::new_signed_with_payer(
                        &[create_lut_ix, extend_ix],
                        Some(&payer.pubkey()),
                        &[&payer],
                        rpc_client.get_latest_blockhash()?,
                    );

                    rpc_client.send_and_confirm_transaction(&lut_tx)?;
                    sleep(Duration::from_secs(2));

                    println!(
                        "LUT {} created with {} addresses",
                        lut_address,
                        lut_addresses.len()
                    );

                    lut_address
                };

                // Fetch the custom lookup table
                let lut_account = rpc_client.get_account(&custom_lut)?;
                let lut = AddressLookupTable::deserialize(&lut_account.data)?;
                let new_lut = AddressLookupTableAccount {
                    key: custom_lut,
                    addresses: lut.addresses.to_vec(),
                };

                address_lookup_tables.push(new_lut);

                let recent_blockhash = rpc_client.get_latest_blockhash()?;

                let versioned_message = VersionedMessage::V0(v0::Message::try_compile(
                    &payer.pubkey(),
                    &instructions,
                    &address_lookup_tables,
                    recent_blockhash,
                )?);

                let versioned_tx = VersionedTransaction {
                    signatures: vec![
                        solana_sdk::signature::Signature::default();
                        versioned_message.header().num_required_signatures as usize
                    ],
                    message: versioned_message,
                };

                // Encode and print the versioned transaction
                let mut encoded = Vec::new();
                versioned_tx.encode(&mut encoded)?;
                let base64_tx = BASE64_STANDARD.encode(&encoded);
                println!("{}", base64_tx);
            }
        }
        _ => panic!("Expected resolved result"),
    }

    Ok(())
}

fn create_transaction(
    vaa_body: Vec<u8>,
    rpc_client: &RpcClient,
    requested_accounts: Vec<AccountMeta>,
    payer: Pubkey,
) -> Result<Transaction, anyhow::Error> {
    let mut instruction_data = Vec::new();
    instruction_data.extend_from_slice(&[148, 184, 169, 222, 207, 8, 154, 127]);
    instruction_data.extend_from_slice(&(vaa_body.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(&vaa_body);

    let mut accounts = vec![AccountMeta::new(payer, true)];
    accounts.extend(requested_accounts);

    let (derived_guardian_set, _) = Pubkey::find_program_address(
        &[GUARDIAN_SET_SEED, &GUARDIAN_SET_INDEX_SEED.to_be_bytes()],
        &CORE_BRIDGE_PROGRAM_ID,
    );

    accounts.push(AccountMeta::new_readonly(derived_guardian_set, false));

    let instruction = SolanaInstruction {
        program_id: wormhole_adapter::ID,
        accounts,
        data: instruction_data,
    };

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let message = Message::new_with_blockhash(&[instruction], None, &recent_blockhash);
    let tx = Transaction::new_unsigned(message);

    let mut encoded = Vec::new();
    let _ = tx.encode(&mut encoded);
    let base64_tx = BASE64_STANDARD.encode(&encoded);
    println!("\n=== Base64 Transaction ===\n{}", base64_tx);

    Ok(tx)
}
