use anchor_client::solana_account_decoder::{UiAccountData, UiAccountEncoding};
use anchor_lang::pubkey;
use anyhow::{Context, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use borsh::BorshDeserialize;
use common::{
    pda,
    wormhole_adapter::{self, constants::GUARDIAN_SET_SEED},
};
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
use std::{collections::HashSet, thread::sleep, time::Duration};

use crate::types::*;

const GUARDIAN_SET_INDEX_SEED: u32 = 0;
const CORE_BRIDGE_PROGRAM_ID: Pubkey = pubkey!("3u8hJUVTA4jH1wYAyUur7FFZVQ8H635K3tSHHF4ssjQ5");
const TARGET_PROGRAM_ID: Pubkey = pubkey!("EFaNWErqAtVWufdNb7yofSHHfWFos843DFpu4JBw24at");
const CUSTOM_LUT: Option<Pubkey> = Some(pubkey!("49zK4hBSgwoNCukBTtGVLkQEr7NUkCfebTb7dqM7uhmG"));
const RPC_URL: &str = "https://lindsy-gxe51w-fast-devnet.helius-rpc.com";
const WORMHOLE_API_URL: &str = "https://api.testnet.wormholescan.io/api/v1/operations";
const EXECUTOR_API_URL: &str = "https://executor-testnet.labsapis.com/v0/status/tx";
const RESOLVER_PUBKEY_SHIM_VAA_SIGS: Pubkey =
    Pubkey::new_from_array(*b"shim_vaa_sigs_000000000000000000");

pub fn resolve_execute(tx_hash: String) -> Result<()> {
    let rpc_client = RpcClient::new(RPC_URL);
    let payer = load_keypair()?;

    // Fetch VAA from Wormhole API
    let vaa_body = fetch_vaa_body(&tx_hash)?;

    // Fetch and extract post VAA instruction
    let (extracted_instruction, guardian_set) =
        extract_target_instruction(&rpc_client, &fetch_executor_transactions(&tx_hash)?)?;

    // Resolve missing accounts through simulation
    let result_pda = pda!(
        &[b"executor-account-resolver:result"],
        &wormhole_adapter::ID
    );
    let requested_accounts = resolve_accounts(&rpc_client, &vaa_body, &payer, &result_pda)?;

    // Get final resolver result
    let resolver_result = get_resolver_result(
        &rpc_client,
        &vaa_body,
        &payer,
        &result_pda,
        requested_accounts,
    )?;

    // Build and output versioned transactions
    build_and_output_transactions(
        &rpc_client,
        &payer,
        resolver_result,
        extracted_instruction,
        guardian_set,
    )?;

    Ok(())
}

fn load_keypair() -> Result<Keypair> {
    let key_path = format!("{}/.config/solana/id.json", std::env::var("HOME")?);
    Keypair::read_from_file(&key_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair from {}: {}", key_path, e))
}

fn fetch_vaa_body(tx_hash: &str) -> Result<Vec<u8>> {
    let api_client = reqwest::blocking::Client::new();
    let url = format!("{}?txHash={}", WORMHOLE_API_URL, tx_hash);

    let response: WormholeResponse = api_client
        .get(&url)
        .send()
        .context("Failed to fetch VAA from Wormhole API")?
        .json()
        .context("Failed to parse Wormhole API response")?;

    let vaa_operation = response
        .operations
        .first()
        .context("No executor operations returned")?;

    let vaa_bytes = BASE64_STANDARD
        .decode(&vaa_operation.vaa.raw)
        .context("Failed to decode VAA bytes")?;

    // Extract VAA body by skipping the header (6 + num_signatures * 66 bytes)
    let header_len = 6 + vaa_bytes[5] as usize * 66;
    Ok(vaa_bytes[header_len..].to_vec())
}

fn fetch_executor_transactions(tx_hash: &str) -> Result<ExecutorTransactions> {
    let api_client = reqwest::blocking::Client::new();
    let body = serde_json::json!({ "txHash": tx_hash });

    api_client
        .post(EXECUTOR_API_URL)
        .json(&body)
        .send()
        .context("Failed to fetch executor transactions")?
        .json()
        .context("Failed to parse executor transactions")
}

fn extract_target_instruction(
    rpc_client: &RpcClient,
    executor_txs: &ExecutorTransactions,
) -> Result<(SolanaInstruction, Pubkey)> {
    let executor_tx = executor_txs.first().context("No executor transactions")?;
    let tx = executor_tx
        .txs
        .first()
        .context("No transactions in executor")?;
    let tx_hash_sig = tx.tx_hash.parse().context("Failed to parse tx hash")?;

    let transaction = rpc_client
        .get_transaction_with_config(
            &tx_hash_sig,
            solana_rpc_client_types::config::RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Json),
                commitment: None,
                max_supported_transaction_version: Some(0),
            },
        )
        .context("Failed to fetch transaction")?;

    let EncodedTransaction::Json(ui_tx) = transaction.transaction.transaction else {
        anyhow::bail!("Expected JSON encoded transaction");
    };

    let UiMessage::Raw(raw_msg) = ui_tx.message else {
        anyhow::bail!("Expected raw message");
    };

    // Find the target program instruction
    for ix in raw_msg.instructions {
        let program_id_index: usize = ix.program_id_index.into();
        let program_id_str = raw_msg
            .account_keys
            .get(program_id_index)
            .context("Invalid program ID index")?;
        let program_id = program_id_str.parse::<Pubkey>()?;

        if program_id == TARGET_PROGRAM_ID {
            let num_required_signatures: usize = raw_msg.header.num_required_signatures.into();
            let num_readonly_signed: usize = raw_msg.header.num_readonly_signed_accounts.into();
            let num_readonly_unsigned: usize = raw_msg.header.num_readonly_unsigned_accounts.into();

            let accounts: Vec<AccountMeta> = ix
                .accounts
                .iter()
                .map(|&idx| {
                    let idx_usize: usize = idx.into();
                    let pubkey = raw_msg.account_keys[idx_usize].parse::<Pubkey>().unwrap();

                    let is_signer = idx_usize < num_required_signatures;
                    let is_writable = if is_signer {
                        idx_usize < num_required_signatures - num_readonly_signed
                    } else {
                        idx_usize < raw_msg.account_keys.len() - num_readonly_unsigned
                    };

                    AccountMeta {
                        pubkey,
                        is_signer,
                        is_writable,
                    }
                })
                .collect();

            let guardian_set = accounts
                .get(1)
                .context("Missing guardian set account")?
                .pubkey;

            // Post VAA instruction
            let instruction = SolanaInstruction {
                program_id,
                accounts,
                data: bs58::decode(&ix.data)
                    .into_vec()
                    .context("Failed to decode instruction data")?,
            };

            return Ok((instruction, guardian_set));
        }
    }

    anyhow::bail!("Target program instruction not found")
}

fn resolve_accounts(
    rpc_client: &RpcClient,
    vaa_body: &[u8],
    payer: &Keypair,
    result_pda: &Pubkey,
) -> Result<Vec<AccountMeta>> {
    let mut requested_accounts = vec![];

    loop {
        let transaction = create_transaction(
            vaa_body.to_vec(),
            rpc_client,
            requested_accounts.clone(),
            payer.pubkey(),
        )?;

        let simulation_result = rpc_client
            .simulate_transaction_with_config(
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
            )
            .context("Failed to simulate transaction")?;

        let return_data = simulation_result
            .value
            .return_data
            .context("No return data from simulation")?;

        let data_bytes = BASE64_STANDARD
            .decode(&return_data.data.0)
            .context("Failed to decode return data")?;

        // Check if we have all accounts (first byte == 1 means missing accounts)
        if data_bytes[0] != 1 {
            break;
        }

        let missing_accounts = parse_missing_accounts(&data_bytes)?;
        println!("=== Missing Accounts ===");
        for (i, pubkey) in missing_accounts.iter().enumerate() {
            println!("Account {}: {}", i, pubkey);
        }

        requested_accounts.extend(missing_accounts.into_iter().map(|pubkey| AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: pubkey == *result_pda,
        }));
    }

    Ok(requested_accounts)
}

fn parse_missing_accounts(data_bytes: &[u8]) -> Result<Vec<Pubkey>> {
    let accounts_len = u32::from_le_bytes(
        data_bytes[1..5]
            .try_into()
            .context("Invalid account length bytes")?,
    ) as usize;

    let mut missing_accounts = Vec::new();
    let mut offset = 5;

    for _ in 0..accounts_len {
        if offset + 32 > data_bytes.len() {
            break;
        }
        let pubkey_bytes = &data_bytes[offset..offset + 32];
        let pubkey = Pubkey::try_from(pubkey_bytes).context("Invalid pubkey bytes")?;
        missing_accounts.push(pubkey);
        offset += 32;
    }

    Ok(missing_accounts)
}

fn get_resolver_result(
    rpc_client: &RpcClient,
    vaa_body: &[u8],
    payer: &Keypair,
    result_pda: &Pubkey,
    requested_accounts: Vec<AccountMeta>,
) -> Result<Resolver<InstructionGroups>> {
    let transaction = create_transaction(
        vaa_body.to_vec(),
        rpc_client,
        requested_accounts,
        payer.pubkey(),
    )?;

    let simulation_result = rpc_client
        .simulate_transaction_with_config(
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
        )
        .context("Failed to get final simulation result")?;

    let result_data = simulation_result
        .value
        .accounts
        .and_then(|accounts| accounts.into_iter().next().flatten())
        .context("Expected final result data")?;

    let decoded_data = match result_data.data {
        UiAccountData::Binary(encoded, _) => BASE64_STANDARD
            .decode(encoded)
            .context("Failed to decode result data")?,
        _ => anyhow::bail!("Unexpected account data encoding"),
    };

    // Skip the 8-byte discriminator and deserialize
    Resolver::<InstructionGroups>::deserialize(&mut &decoded_data[8..])
        .context("Failed to deserialize resolver result")
}

fn build_and_output_transactions(
    rpc_client: &RpcClient,
    payer: &Keypair,
    resolver_result: Resolver<InstructionGroups>,
    extracted_instruction: SolanaInstruction,
    guardian_set: Pubkey,
) -> Result<()> {
    let Resolver::Resolved(instruction_groups) = resolver_result else {
        anyhow::bail!("Expected resolved result");
    };

    println!("\n=== Resolver Result ===");

    for group in instruction_groups.0.iter() {
        let mut instructions = vec![extracted_instruction.clone()];

        // Build instructions from the resolver output
        for serializable_ix in group.instructions.iter() {
            let accounts: Vec<AccountMeta> = serializable_ix
                .accounts
                .iter()
                .map(|acc| AccountMeta {
                    pubkey: if acc.pubkey == RESOLVER_PUBKEY_SHIM_VAA_SIGS {
                        guardian_set
                    } else {
                        acc.pubkey
                    },
                    is_signer: acc.is_signer,
                    is_writable: acc.is_writable,
                })
                .collect();

            instructions.push(SolanaInstruction {
                program_id: serializable_ix.program_id,
                accounts,
                data: serializable_ix.data.clone(),
            });
        }

        // Fetch address lookup tables
        let mut lut_pubkeys = group.address_lookup_tables.clone();

        // Get or create custom LUT and add it to the list
        let custom_lut = get_or_create_lut(rpc_client, payer, &instructions)?;
        lut_pubkeys.push(custom_lut);

        let address_lookup_tables = fetch_lookup_tables(rpc_client, &lut_pubkeys)?;

        // Build and output versioned transaction
        let versioned_tx =
            build_versioned_transaction(rpc_client, payer, &instructions, &address_lookup_tables)?;

        let mut encoded = Vec::new();
        versioned_tx.encode(&mut encoded)?;
        let base64_tx = BASE64_STANDARD.encode(&encoded);
        println!("{}", base64_tx);
    }

    Ok(())
}

fn fetch_lookup_tables(
    rpc_client: &RpcClient,
    lut_pubkeys: &[Pubkey],
) -> Result<Vec<AddressLookupTableAccount>> {
    let mut lookup_tables = Vec::new();

    for lut_pubkey in lut_pubkeys {
        let lut_account = rpc_client
            .get_account(lut_pubkey)
            .with_context(|| format!("Failed to fetch lookup table {}", lut_pubkey))?;

        let lut = AddressLookupTable::deserialize(&lut_account.data)
            .with_context(|| format!("Failed to deserialize lookup table {}", lut_pubkey))?;

        lookup_tables.push(AddressLookupTableAccount {
            key: *lut_pubkey,
            addresses: lut.addresses.to_vec(),
        });
    }

    Ok(lookup_tables)
}

fn get_or_create_lut(
    rpc_client: &RpcClient,
    payer: &Keypair,
    instructions: &[SolanaInstruction],
) -> Result<Pubkey> {
    if let Some(lut) = CUSTOM_LUT {
        return Ok(lut);
    }

    // Collect all unique accounts from instructions
    let all_accounts: HashSet<Pubkey> = instructions
        .iter()
        .flat_map(|ix| ix.accounts.iter().map(|acc| acc.pubkey))
        .collect();

    let lut_addresses: Vec<Pubkey> = all_accounts.into_iter().collect();
    let recent_slot = rpc_client
        .get_slot_with_commitment(CommitmentConfig {
            commitment: CommitmentLevel::Finalized,
        })
        .context("Failed to get recent slot")?;

    let (create_lut_ix, lut_address) =
        solana_sdk::address_lookup_table::instruction::create_lookup_table(
            payer.pubkey(),
            payer.pubkey(),
            recent_slot - 50,
        );

    let extend_ix = solana_sdk::address_lookup_table::instruction::extend_lookup_table(
        lut_address,
        payer.pubkey(),
        Some(payer.pubkey()),
        lut_addresses.clone(),
    );

    let lut_tx = Transaction::new_signed_with_payer(
        &[create_lut_ix, extend_ix],
        Some(&payer.pubkey()),
        &[payer],
        rpc_client
            .get_latest_blockhash()
            .context("Failed to get latest blockhash")?,
    );

    rpc_client
        .send_and_confirm_transaction(&lut_tx)
        .context("Failed to send LUT transaction")?;

    sleep(Duration::from_secs(2));

    println!(
        "LUT {} created with {} addresses",
        lut_address,
        lut_addresses.len()
    );

    Ok(lut_address)
}

fn build_versioned_transaction(
    rpc_client: &RpcClient,
    payer: &Keypair,
    instructions: &[SolanaInstruction],
    address_lookup_tables: &[AddressLookupTableAccount],
) -> Result<VersionedTransaction> {
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .context("Failed to get latest blockhash")?;

    let versioned_message = VersionedMessage::V0(
        v0::Message::try_compile(
            &payer.pubkey(),
            instructions,
            address_lookup_tables,
            recent_blockhash,
        )
        .context("Failed to compile versioned message")?,
    );

    Ok(VersionedTransaction {
        signatures: vec![
            solana_sdk::signature::Signature::default();
            versioned_message.header().num_required_signatures as usize
        ],
        message: versioned_message,
    })
}

fn create_transaction(
    vaa_body: Vec<u8>,
    rpc_client: &RpcClient,
    requested_accounts: Vec<AccountMeta>,
    payer: Pubkey,
) -> Result<Transaction> {
    // Build instruction data: discriminator + VAA length + VAA body
    let mut instruction_data = Vec::new();
    instruction_data.extend_from_slice(&[148, 184, 169, 222, 207, 8, 154, 127]);
    instruction_data.extend_from_slice(&(vaa_body.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(&vaa_body);

    // Build accounts list
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

    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .context("Failed to get latest blockhash")?;

    let message = Message::new_with_blockhash(&[instruction], None, &recent_blockhash);
    let tx = Transaction::new_unsigned(message);

    // Debug output
    let mut encoded = Vec::new();
    tx.encode(&mut encoded)?;
    let base64_tx = BASE64_STANDARD.encode(&encoded);
    println!("\n=== Base64 Transaction ===\n{}", base64_tx);

    Ok(tx)
}
