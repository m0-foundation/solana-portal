use crate::types::*;
use anchor_client::solana_account_decoder::{UiAccountData, UiAccountEncoding};
use anchor_lang::pubkey;
use anyhow::{Context, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use borsh::BorshDeserialize;
use m0_portal_common::{
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

    // Fetch VAA and the post instruction using the Wormhole API
    let vaa_body = fetch_vaa_body(&tx_hash)?;
    let (instruction, guardian_set) =
        extract_target_instruction(&rpc_client, &fetch_executor_transactions(&tx_hash)?)?;

    // Simulate until all accounts are resolved and result PDA is populated
    let result_pda = pda!(
        &[b"executor-account-resolver:result"],
        &wormhole_adapter::ID
    );
    let requested_accounts = resolve_accounts(&rpc_client, &vaa_body, &payer, &result_pda)?;

    let resolver_result = get_resolver_result(
        &rpc_client,
        &vaa_body,
        &payer,
        &result_pda,
        requested_accounts,
    )?;

    build_and_output_transactions(
        &rpc_client,
        &payer,
        resolver_result,
        instruction,
        guardian_set,
    )
}

fn load_keypair() -> Result<Keypair> {
    let key_path = format!("{}/.config/solana/id.json", std::env::var("HOME")?);
    Keypair::read_from_file(&key_path).map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))
}

fn fetch_vaa_body(tx_hash: &str) -> Result<Vec<u8>> {
    let response: WormholeResponse = reqwest::blocking::Client::new()
        .get(format!("{}?txHash={}", WORMHOLE_API_URL, tx_hash))
        .send()
        .context("Failed to fetch VAA")?
        .json()
        .context("Failed to parse VAA response")?;

    let vaa_bytes = BASE64_STANDARD.decode(
        &response
            .operations
            .first()
            .context("No operations")?
            .vaa
            .raw,
    )?;
    let header_len = 6 + vaa_bytes[5] as usize * 66;
    Ok(vaa_bytes[header_len..].to_vec())
}

fn fetch_executor_transactions(tx_hash: &str) -> Result<ExecutorTransactions> {
    reqwest::blocking::Client::new()
        .post(EXECUTOR_API_URL)
        .json(&serde_json::json!({ "txHash": tx_hash }))
        .send()
        .context("Failed to fetch executor txs")?
        .json()
        .context("Failed to parse executor txs")
}

fn extract_target_instruction(
    rpc_client: &RpcClient,
    executor_txs: &ExecutorTransactions,
) -> Result<(SolanaInstruction, Pubkey)> {
    let tx_hash_sig = executor_txs
        .first()
        .context("No executor txs")?
        .txs
        .first()
        .context("No txs")?
        .tx_hash
        .parse()?;

    let transaction = rpc_client.get_transaction_with_config(
        &tx_hash_sig,
        solana_rpc_client_types::config::RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Json),
            commitment: None,
            max_supported_transaction_version: Some(0),
        },
    )?;

    let EncodedTransaction::Json(ui_tx) = transaction.transaction.transaction else {
        anyhow::bail!("Expected JSON encoded transaction");
    };
    let UiMessage::Raw(raw_msg) = ui_tx.message else {
        anyhow::bail!("Expected raw message");
    };

    for ix in raw_msg.instructions {
        let program_id: Pubkey = raw_msg
            .account_keys
            .get(ix.program_id_index as usize)
            .context("Invalid program ID")?
            .parse()?;

        if program_id == TARGET_PROGRAM_ID {
            let (num_sigs, num_ro_signed, num_ro_unsigned) = (
                raw_msg.header.num_required_signatures as usize,
                raw_msg.header.num_readonly_signed_accounts as usize,
                raw_msg.header.num_readonly_unsigned_accounts as usize,
            );

            let accounts: Vec<AccountMeta> = ix
                .accounts
                .iter()
                .map(|&idx| {
                    let idx = idx as usize;
                    let pubkey = raw_msg.account_keys[idx].parse::<Pubkey>().unwrap();
                    let is_signer = idx < num_sigs;
                    let is_writable = if is_signer {
                        idx < num_sigs - num_ro_signed
                    } else {
                        idx < raw_msg.account_keys.len() - num_ro_unsigned
                    };
                    AccountMeta {
                        pubkey,
                        is_signer,
                        is_writable,
                    }
                })
                .collect();

            return Ok((
                SolanaInstruction {
                    program_id,
                    accounts: accounts.clone(),
                    data: bs58::decode(&ix.data).into_vec()?,
                },
                accounts.get(1).context("Missing guardian set")?.pubkey,
            ));
        }
    }
    anyhow::bail!("Target program not found")
}

fn resolve_accounts(
    rpc_client: &RpcClient,
    vaa_body: &[u8],
    payer: &Keypair,
    result_pda: &Pubkey,
) -> Result<Vec<AccountMeta>> {
    let mut requested_accounts = vec![];

    loop {
        let tx = create_transaction(
            vaa_body.to_vec(),
            rpc_client,
            requested_accounts.clone(),
            payer.pubkey(),
        )?;
        let sim = rpc_client.simulate_transaction_with_config(
            &tx,
            RpcSimulateTransactionConfig {
                sig_verify: false,
                replace_recent_blockhash: true,
                accounts: Some(RpcSimulateTransactionAccountsConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    addresses: vec![result_pda.to_string()],
                }),
                ..Default::default()
            },
        )?;

        let data = sim.value.return_data.context("No return data")?.data;
        let data_bytes = BASE64_STANDARD.decode(&data.0)?;
        if data_bytes[0] != 1 {
            break;
        }

        let missing = parse_missing_accounts(&data_bytes)?;
        println!("=== Missing Accounts ===");
        missing
            .iter()
            .enumerate()
            .for_each(|(i, pk)| println!("Account {}: {}", i, pk));

        requested_accounts.extend(missing.into_iter().map(|pubkey| AccountMeta {
            pubkey,
            is_signer: false,
            is_writable: pubkey == *result_pda,
        }));
    }

    Ok(requested_accounts)
}

fn parse_missing_accounts(data: &[u8]) -> Result<Vec<Pubkey>> {
    let len = u32::from_le_bytes(data[1..5].try_into()?) as usize;
    Ok((0..len)
        .filter_map(|i| {
            let offset = 5 + i * 32;
            (offset + 32 <= data.len())
                .then(|| Pubkey::try_from(&data[offset..offset + 32]).ok())
                .flatten()
        })
        .collect())
}

fn get_resolver_result(
    rpc_client: &RpcClient,
    vaa_body: &[u8],
    payer: &Keypair,
    result_pda: &Pubkey,
    requested_accounts: Vec<AccountMeta>,
) -> Result<Resolver<InstructionGroups>> {
    let tx = create_transaction(
        vaa_body.to_vec(),
        rpc_client,
        requested_accounts,
        payer.pubkey(),
    )?;

    let sim = rpc_client.simulate_transaction_with_config(
        &tx,
        RpcSimulateTransactionConfig {
            sig_verify: false,
            replace_recent_blockhash: true,
            accounts: Some(RpcSimulateTransactionAccountsConfig {
                encoding: Some(UiAccountEncoding::Base64),
                addresses: vec![result_pda.to_string()],
            }),
            ..Default::default()
        },
    )?;

    let result_data = sim
        .value
        .accounts
        .and_then(|a| a.into_iter().next().flatten())
        .context("No result data")?;

    let decoded = match result_data.data {
        UiAccountData::Binary(e, _) => BASE64_STANDARD.decode(e)?,
        _ => anyhow::bail!("Unexpected encoding"),
    };

    Resolver::<InstructionGroups>::deserialize(&mut &decoded[8..]).context("Failed to deserialize")
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
        instructions.extend(group.instructions.iter().map(|ix| {
            SolanaInstruction {
                program_id: ix.program_id,
                accounts: ix
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
                    .collect(),
                data: ix.data.clone(),
            }
        }));

        let mut lut_pubkeys = group.address_lookup_tables.clone();
        lut_pubkeys.push(get_or_create_lut(rpc_client, payer, &instructions)?);
        let luts = fetch_lookup_tables(rpc_client, &lut_pubkeys)?;
        let vtx = build_versioned_transaction(rpc_client, payer, &instructions, &luts)?;

        let mut encoded = Vec::new();
        vtx.encode(&mut encoded)?;
        println!("{}", BASE64_STANDARD.encode(&encoded));
    }

    Ok(())
}

fn fetch_lookup_tables(
    rpc_client: &RpcClient,
    lut_pubkeys: &[Pubkey],
) -> Result<Vec<AddressLookupTableAccount>> {
    lut_pubkeys
        .iter()
        .map(|key| {
            let acc = rpc_client.get_account(key)?;
            let lut = AddressLookupTable::deserialize(&acc.data)?;
            Ok(AddressLookupTableAccount {
                key: *key,
                addresses: lut.addresses.to_vec(),
            })
        })
        .collect()
}

fn get_or_create_lut(
    rpc_client: &RpcClient,
    payer: &Keypair,
    instructions: &[SolanaInstruction],
) -> Result<Pubkey> {
    if let Some(lut) = CUSTOM_LUT {
        return Ok(lut);
    }

    let addresses: Vec<Pubkey> = instructions
        .iter()
        .flat_map(|ix| ix.accounts.iter().map(|acc| acc.pubkey))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let slot = rpc_client.get_slot_with_commitment(CommitmentConfig {
        commitment: CommitmentLevel::Finalized,
    })?;
    let (create_ix, lut_addr) = solana_sdk::address_lookup_table::instruction::create_lookup_table(
        payer.pubkey(),
        payer.pubkey(),
        slot - 50,
    );
    let extend_ix = solana_sdk::address_lookup_table::instruction::extend_lookup_table(
        lut_addr,
        payer.pubkey(),
        Some(payer.pubkey()),
        addresses.clone(),
    );

    rpc_client.send_and_confirm_transaction(&Transaction::new_signed_with_payer(
        &[create_ix, extend_ix],
        Some(&payer.pubkey()),
        &[payer],
        rpc_client.get_latest_blockhash()?,
    ))?;

    // Wait for LUT to be picked up
    sleep(Duration::from_secs(2));

    println!(
        "LUT {} created with {} addresses",
        lut_addr,
        addresses.len()
    );

    Ok(lut_addr)
}

fn build_versioned_transaction(
    rpc_client: &RpcClient,
    payer: &Keypair,
    instructions: &[SolanaInstruction],
    address_lookup_tables: &[AddressLookupTableAccount],
) -> Result<VersionedTransaction> {
    let msg = VersionedMessage::V0(v0::Message::try_compile(
        &payer.pubkey(),
        instructions,
        address_lookup_tables,
        rpc_client.get_latest_blockhash()?,
    )?);

    Ok(VersionedTransaction {
        signatures: vec![
            solana_sdk::signature::Signature::default();
            msg.header().num_required_signatures as usize
        ],
        message: msg,
    })
}

fn create_transaction(
    vaa_body: Vec<u8>,
    rpc_client: &RpcClient,
    requested_accounts: Vec<AccountMeta>,
    payer: Pubkey,
) -> Result<Transaction> {
    let mut data = vec![148, 184, 169, 222, 207, 8, 154, 127];
    data.extend_from_slice(&(vaa_body.len() as u32).to_le_bytes());
    data.extend_from_slice(&vaa_body);

    let mut accounts = vec![AccountMeta::new(payer, true)];
    accounts.extend(requested_accounts);
    accounts.push(AccountMeta::new_readonly(
        Pubkey::find_program_address(
            &[GUARDIAN_SET_SEED, &GUARDIAN_SET_INDEX_SEED.to_be_bytes()],
            &CORE_BRIDGE_PROGRAM_ID,
        )
        .0,
        false,
    ));

    let tx = Transaction::new_unsigned(Message::new_with_blockhash(
        &[SolanaInstruction {
            program_id: wormhole_adapter::ID,
            accounts,
            data,
        }],
        None,
        &rpc_client.get_latest_blockhash()?,
    ));

    let mut encoded = Vec::new();
    tx.encode(&mut encoded)?;
    println!(
        "\n=== Base64 Transaction ===\n{}",
        BASE64_STANDARD.encode(&encoded)
    );

    Ok(tx)
}
