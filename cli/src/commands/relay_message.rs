use anchor_lang::AnchorDeserialize;
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use executor_account_resolver_svm::{
    InstructionGroups, Resolver, RESOLVER_PUBKEY_GUARDIAN_SET, RESOLVER_PUBKEY_PAYER,
    RESOLVER_PUBKEY_SHIM_VAA_SIGS,
};
use m0_portal_common::{pda, wormhole_adapter, wormhole_verify_vaa_shim};
use serde::Deserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    address_lookup_table::state::AddressLookupTable,
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    message::{v0, AddressLookupTableAccount, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::VersionedTransaction,
};
use solana_rpc_client_types::config::RpcSimulateTransactionConfig;

use super::common::load_keypair;

// WormholeScan API URLs
const WORMHOLESCAN_API_MAINNET: &str = "https://api.wormholescan.io/api/v1/vaas";
const WORMHOLESCAN_API_TESTNET: &str = "https://api.testnet.wormholescan.io/api/v1/vaas";

// Instruction discriminators
const POST_SIGNATURES_DISCRIMINATOR: [u8; 8] = [138, 2, 53, 166, 45, 77, 137, 51];
const CLOSE_SIGNATURES_DISCRIMINATOR: [u8; 8] = [192, 65, 63, 117, 213, 138, 179, 190];
const RESOLVER_EXECUTE_VAA_V1: [u8; 8] = [148, 184, 169, 222, 207, 8, 154, 127];

// Result account seed and discriminator
const RESOLVER_RESULT_ACCOUNT_SEED: &[u8] = b"executor-account-resolver:result";
const RESOLVER_RESULT_ACCOUNT_DISCRIMINATOR: [u8; 8] = [34, 185, 243, 199, 181, 255, 28, 227];

// Transaction configuration
const COMPUTE_UNIT_LIMIT: u32 = 600_000;
const COMPUTE_UNIT_PRICE: u64 = 100_000;

// Core Bridge Program IDs
const CORE_BRIDGE_MAINNET: Pubkey =
    solana_sdk::pubkey!("worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth");
const CORE_BRIDGE_DEVNET: Pubkey =
    solana_sdk::pubkey!("3u8hJUVTA4jH1wYAyUur7FFZVQ8H635K3tSHHF4ssjQ5");

const GUARDIAN_SET_SEED: &[u8] = b"GuardianSet";

const MAX_SIMULATION_ITERATIONS: usize = 10;

#[derive(Deserialize)]
struct WormholeScanResponse {
    data: WormholeScanVaa,
}

#[derive(Deserialize)]
struct WormholeScanVaa {
    vaa: String, // base64-encoded full VAA
}

struct ParsedVaa {
    guardian_set_index: u32,
    signatures: Vec<[u8; 66]>,
    body: Vec<u8>,
}

pub async fn relay_message(vaa_id: String, rpc_url: Option<String>, testnet: bool) -> Result<()> {
    let payer = load_keypair()?;
    println!("Payer: {}", payer.pubkey());

    let rpc_url = rpc_url.unwrap_or_else(|| {
        std::env::var("RPC_URL").unwrap_or_else(|_| {
            if testnet {
                "https://api.devnet.solana.com".to_string()
            } else {
                "https://api.mainnet-beta.solana.com".to_string()
            }
        })
    });
    let rpc = RpcClient::new(rpc_url.clone());
    println!("RPC: {}", rpc_url);

    let core_bridge = if testnet {
        CORE_BRIDGE_DEVNET
    } else {
        CORE_BRIDGE_MAINNET
    };

    // Step 1: Fetch VAA from WormholeScan
    println!("\n--- Step 1: Fetching VAA from WormholeScan ---");
    let vaa_bytes = fetch_vaa(&vaa_id, testnet).await?;
    println!("Fetched VAA: {} bytes", vaa_bytes.len());

    // Step 2: Parse VAA
    println!("\n--- Step 2: Parsing VAA ---");
    let parsed = parse_vaa(&vaa_bytes)?;
    println!(
        "Guardian set index: {}, Signatures: {}, Body: {} bytes",
        parsed.guardian_set_index,
        parsed.signatures.len(),
        parsed.body.len()
    );

    // Step 3: Post guardian signatures
    println!("\n--- Step 3: Posting guardian signatures ---");
    let guardian_sigs_keypair = Keypair::new();
    println!(
        "Guardian signatures account: {}",
        guardian_sigs_keypair.pubkey()
    );

    let post_sig_ix = build_post_signatures_ix(
        &payer.pubkey(),
        &guardian_sigs_keypair.pubkey(),
        parsed.guardian_set_index,
        &parsed.signatures,
    );

    let recent_blockhash = rpc.get_latest_blockhash().await?;
    let post_sig_msg = solana_sdk::message::Message::new(&[post_sig_ix], Some(&payer.pubkey()));
    let post_sig_tx = solana_sdk::transaction::Transaction::new(
        &[&payer, &guardian_sigs_keypair],
        post_sig_msg,
        recent_blockhash,
    );

    let post_sig_signature = rpc
        .send_and_confirm_transaction(&post_sig_tx)
        .await
        .context("Failed to post guardian signatures")?;
    println!("Posted signatures: {}", post_sig_signature);

    // Step 4: Resolve accounts via simulation
    println!("\n--- Step 4: Resolving accounts via simulation ---");
    let guardian_set_pda = Pubkey::find_program_address(
        &[
            GUARDIAN_SET_SEED,
            &parsed.guardian_set_index.to_be_bytes(),
        ],
        &core_bridge,
    )
    .0;
    println!("Guardian set PDA: {}", guardian_set_pda);

    let resolved = resolve_accounts(
        &rpc,
        &payer,
        &parsed.body,
        guardian_set_pda,
    )
    .await?;

    // Step 5: Build and submit receive_message transaction
    println!("\n--- Step 5: Building receive_message transaction ---");
    let (instruction_groups, lut_keys) = match resolved {
        Resolver::Resolved(groups) => {
            let luts: Vec<Pubkey> = groups
                .0
                .iter()
                .flat_map(|g| g.address_lookup_tables.clone())
                .collect();
            (groups, luts)
        }
        Resolver::Missing(missing) => {
            anyhow::bail!(
                "Resolution incomplete. Missing accounts: {:?}",
                missing.accounts
            );
        }
        Resolver::Account() => {
            anyhow::bail!("Unexpected Resolver::Account variant");
        }
    };

    // Extract the receive_message instruction from resolved groups
    let group = instruction_groups
        .0
        .first()
        .context("No instruction groups returned from resolver")?;

    let serialized_ix = group
        .instructions
        .first()
        .context("No instructions in resolved group")?;

    // Convert SerializableInstruction to Instruction, replacing placeholders
    let receive_ix = build_receive_ix(
        serialized_ix,
        &payer.pubkey(),
        &guardian_sigs_keypair.pubkey(),
        guardian_set_pda,
    );

    // Build close_signatures instruction
    let close_sigs_ix = build_close_signatures_ix(
        &guardian_sigs_keypair.pubkey(),
        &payer.pubkey(),
    );

    // Build versioned transaction with LUTs
    let instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(COMPUTE_UNIT_LIMIT),
        ComputeBudgetInstruction::set_compute_unit_price(COMPUTE_UNIT_PRICE),
        receive_ix,
        close_sigs_ix,
    ];

    let tx = build_versioned_tx(
        &rpc,
        instructions,
        &lut_keys,
        &payer,
        None,
    )
    .await?;

    let signature = rpc
        .send_and_confirm_transaction(&tx)
        .await
        .context("Failed to send receive_message transaction")?;

    println!("\n--- Done ---");
    println!("Receive message signature: {}", signature);

    Ok(())
}

async fn fetch_vaa(vaa_id: &str, testnet: bool) -> Result<Vec<u8>> {
    let parts: Vec<&str> = vaa_id.split('/').collect();
    if parts.len() != 3 {
        anyhow::bail!(
            "Invalid VAA ID format. Expected: chain/emitter/sequence, got: {}",
            vaa_id
        );
    }

    let base_url = if testnet {
        WORMHOLESCAN_API_TESTNET
    } else {
        WORMHOLESCAN_API_MAINNET
    };

    let url = format!("{}/{}/{}/{}", base_url, parts[0], parts[1], parts[2]);
    println!("Fetching: {}", url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch VAA from WormholeScan")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "WormholeScan API error: {} {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
    }

    let scan_response: WormholeScanResponse = response
        .json()
        .await
        .context("Failed to parse WormholeScan response")?;

    let vaa_bytes = BASE64
        .decode(&scan_response.data.vaa)
        .context("Failed to base64-decode VAA")?;

    Ok(vaa_bytes)
}

fn parse_vaa(vaa_bytes: &[u8]) -> Result<ParsedVaa> {
    if vaa_bytes.is_empty() {
        anyhow::bail!("VAA bytes are empty");
    }

    let version = vaa_bytes[0];
    if version != 1 {
        anyhow::bail!("Unsupported VAA version: {} (expected 1)", version);
    }

    if vaa_bytes.len() < 6 {
        anyhow::bail!("VAA too short to contain header");
    }

    let guardian_set_index = u32::from_be_bytes(
        vaa_bytes[1..5]
            .try_into()
            .context("Failed to read guardian_set_index")?,
    );

    let num_signatures = vaa_bytes[5] as usize;
    let sigs_end = 6 + num_signatures * 66;

    if vaa_bytes.len() < sigs_end {
        anyhow::bail!(
            "VAA too short for {} signatures (need {} bytes, have {})",
            num_signatures,
            sigs_end,
            vaa_bytes.len()
        );
    }

    let mut signatures = Vec::with_capacity(num_signatures);
    for i in 0..num_signatures {
        let start = 6 + i * 66;
        let mut sig = [0u8; 66];
        sig.copy_from_slice(&vaa_bytes[start..start + 66]);
        signatures.push(sig);
    }

    let body = vaa_bytes[sigs_end..].to_vec();

    Ok(ParsedVaa {
        guardian_set_index,
        signatures,
        body,
    })
}

fn build_post_signatures_ix(
    payer: &Pubkey,
    guardian_sigs_account: &Pubkey,
    guardian_set_index: u32,
    signatures: &[[u8; 66]],
) -> Instruction {
    // Anchor discriminator + guardian_set_index(u32) + total_signatures(u8) + signatures(Vec<[u8;66]>)
    let mut data = Vec::new();
    data.extend_from_slice(&POST_SIGNATURES_DISCRIMINATOR);
    data.extend_from_slice(&guardian_set_index.to_le_bytes());
    data.push(signatures.len() as u8);
    // Vec length prefix (4 bytes LE)
    data.extend_from_slice(&(signatures.len() as u32).to_le_bytes());
    for sig in signatures {
        data.extend_from_slice(sig);
    }

    Instruction {
        program_id: wormhole_verify_vaa_shim::ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(*guardian_sigs_account, true),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data,
    }
}

fn build_close_signatures_ix(
    guardian_sigs_account: &Pubkey,
    refund_recipient: &Pubkey,
) -> Instruction {
    let mut data = Vec::new();
    data.extend_from_slice(&CLOSE_SIGNATURES_DISCRIMINATOR);

    Instruction {
        program_id: wormhole_verify_vaa_shim::ID,
        accounts: vec![
            AccountMeta::new(*guardian_sigs_account, false),
            AccountMeta::new(*refund_recipient, true),
        ],
        data,
    }
}

async fn resolve_accounts(
    rpc: &RpcClient,
    payer: &Keypair,
    vaa_body: &[u8],
    guardian_set_pda: Pubkey,
) -> Result<Resolver<InstructionGroups>> {
    let result_pda = pda!(&[RESOLVER_RESULT_ACCOUNT_SEED], &wormhole_adapter::ID);
    println!("Result PDA: {}", result_pda);

    // Initial accounts for resolve_execute: payer(signer,mut) + guardian_set(readonly)
    let mut account_metas = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(guardian_set_pda, false),
    ];

    // Build instruction data: discriminator(8) + vaa_body as Vec<u8> (4-byte len prefix + bytes)
    let mut ix_data = Vec::new();
    ix_data.extend_from_slice(&RESOLVER_EXECUTE_VAA_V1);
    ix_data.extend_from_slice(&(vaa_body.len() as u32).to_le_bytes());
    ix_data.extend_from_slice(vaa_body);

    for iteration in 0..MAX_SIMULATION_ITERATIONS {
        println!("Simulation iteration {}...", iteration + 1);

        let ix = Instruction {
            program_id: wormhole_adapter::ID,
            accounts: account_metas.clone(),
            data: ix_data.clone(),
        };

        let msg = solana_sdk::message::Message::new(&[ix], Some(&payer.pubkey()));
        let tx = solana_sdk::transaction::Transaction::new_unsigned(msg);

        let sim_config = RpcSimulateTransactionConfig {
            sig_verify: false,
            replace_recent_blockhash: true,
            accounts: Some(solana_rpc_client_types::config::RpcSimulateTransactionAccountsConfig {
                encoding: Some(solana_account_decoder_client_types::UiAccountEncoding::Base64),
                addresses: vec![result_pda.to_string()],
            }),
            ..Default::default()
        };

        let sim_result = rpc
            .simulate_transaction_with_config(
                &tx,
                sim_config,
            )
            .await
            .context("Simulation RPC call failed")?;

        if let Some(err) = &sim_result.value.err {
            // Print logs for debugging
            if let Some(logs) = &sim_result.value.logs {
                for log in logs {
                    println!("  log: {}", log);
                }
            }
            anyhow::bail!("Simulation failed: {:?}", err);
        }

        // Always check return data first to avoid reading stale result account state.
        // The on-chain resolver returns Missing/Resolved inline via return data, or
        // Account() when the result is stored in the result PDA (too large for return data).
        let return_data = sim_result
            .value
            .return_data
            .as_ref()
            .context("No return data from simulation")?;

        let decoded = BASE64
            .decode(&return_data.data.0)
            .context("Failed to decode return data")?;

        let resolver: Resolver<InstructionGroups> =
            AnchorDeserialize::deserialize(&mut decoded.as_slice())
                .context("Failed to deserialize return data as Resolver")?;

        match &resolver {
            Resolver::Missing(missing) => {
                println!(
                    "  Missing {} accounts, {} LUTs",
                    missing.accounts.len(),
                    missing.address_lookup_tables.len()
                );
                for pubkey in &missing.accounts {
                    // Skip placeholders - they don't need to be added as remaining accounts
                    if *pubkey == RESOLVER_PUBKEY_PAYER
                        || *pubkey == RESOLVER_PUBKEY_SHIM_VAA_SIGS
                        || *pubkey == RESOLVER_PUBKEY_GUARDIAN_SET
                    {
                        continue;
                    }
                    println!("    Adding: {}", pubkey);
                    account_metas.push(AccountMeta::new(*pubkey, false));
                }
                // Also fetch and add LUT addresses
                for lut_pubkey in &missing.address_lookup_tables {
                    println!("    Adding LUT: {}", lut_pubkey);
                    account_metas.push(AccountMeta::new_readonly(*lut_pubkey, false));
                }
                continue;
            }
            Resolver::Resolved(_) => {
                println!("  Resolved (inline)!");
                return Ok(resolver);
            }
            Resolver::Account() => {
                // Result is stored in the result PDA account - parse it from simulation state
                println!("  Resolved (via account)!");
                let account_data = sim_result
                    .value
                    .accounts
                    .as_ref()
                    .and_then(|accs| accs.first())
                    .and_then(|acc| acc.as_ref())
                    .context("Result PDA account not returned from simulation")?;

                let data_bytes = account_data
                    .data
                    .decode()
                    .context("Failed to decode result account data")?;

                if data_bytes.len() < 8 {
                    anyhow::bail!(
                        "Result account data too short: {} bytes",
                        data_bytes.len()
                    );
                }

                if data_bytes[..8] != RESOLVER_RESULT_ACCOUNT_DISCRIMINATOR {
                    anyhow::bail!("Result account discriminator mismatch");
                }

                let account_resolver: Resolver<InstructionGroups> =
                    AnchorDeserialize::deserialize(&mut &data_bytes[8..])
                        .context("Failed to deserialize result account as Resolver")?;

                return Ok(account_resolver);
            }
        }
    }

    anyhow::bail!(
        "Account resolution did not converge after {} iterations",
        MAX_SIMULATION_ITERATIONS
    );
}

fn build_receive_ix(
    serialized: &executor_account_resolver_svm::SerializableInstruction,
    payer: &Pubkey,
    guardian_sigs: &Pubkey,
    guardian_set: Pubkey,
) -> Instruction {
    let accounts: Vec<AccountMeta> = serialized
        .accounts
        .iter()
        .map(|a| {
            let pubkey = if a.pubkey == RESOLVER_PUBKEY_PAYER {
                *payer
            } else if a.pubkey == RESOLVER_PUBKEY_SHIM_VAA_SIGS {
                *guardian_sigs
            } else if a.pubkey == RESOLVER_PUBKEY_GUARDIAN_SET {
                guardian_set
            } else {
                a.pubkey
            };

            if a.is_writable {
                AccountMeta::new(pubkey, a.is_signer)
            } else {
                AccountMeta::new_readonly(pubkey, a.is_signer)
            }
        })
        .collect();

    Instruction {
        program_id: serialized.program_id,
        accounts,
        data: serialized.data.clone(),
    }
}

async fn build_versioned_tx(
    rpc: &RpcClient,
    instructions: Vec<Instruction>,
    lut_keys: &[Pubkey],
    payer: &Keypair,
    additional_signer: Option<&Keypair>,
) -> Result<VersionedTransaction> {
    let mut lut_accounts = Vec::new();

    for key in lut_keys {
        let account = rpc
            .get_account(key)
            .await
            .with_context(|| format!("Failed to fetch LUT account: {}", key))?;

        let addresses = AddressLookupTable::deserialize(&account.data)?
            .addresses
            .to_vec();

        lut_accounts.push(AddressLookupTableAccount {
            key: *key,
            addresses,
        });
    }

    let recent_blockhash = rpc.get_latest_blockhash().await?;

    let message = v0::Message::try_compile(
        &payer.pubkey(),
        &instructions,
        &lut_accounts,
        recent_blockhash,
    )?;

    let versioned_message = VersionedMessage::V0(message);

    let mut signers: Vec<&Keypair> = vec![payer];
    if let Some(extra) = additional_signer {
        signers.push(extra);
    }

    let signers_refs: Vec<&dyn Signer> = signers.iter().map(|k| *k as &dyn Signer).collect();
    Ok(VersionedTransaction::try_new(
        versioned_message,
        &signers_refs,
    )?)
}
