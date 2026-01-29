use anchor_lang::{system_program, AccountDeserialize};
use anyhow::{Context, Result};
use m0_portal_common::{
    build_relay_instruction, get_current_sequence_blocking, get_wormhole_chain_id,
    hyperlane_adapter::{
        self,
        accounts::{HyperlaneGlobal, HyperlaneUserGlobal},
        constants::DASH_SEED,
    },
    pda,
    portal::{self, constants::GLOBAL_SEED},
    wormhole_adapter::{self},
    HyperlaneRemainingAccounts, WormholeRemainingAccounts, AUTHORITY_SEED,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction as SolanaInstruction},
    signature::Keypair,
    signer::{EncodableKey, Signer},
    transaction::Transaction,
};

use crate::{types::calculate_instruction_discriminator, BridgeAdapter};

// Hyperlane Testnet values
const TESTNET_RPC_URL: &str = "https://api.testnet.solana.com";

// Wormhole Devnet values
const DEVNET_RPC_URL: &str = "https://api.devnet.solana.com";

pub fn send_index(destination_chain_id: u32, adapter: BridgeAdapter) -> Result<()> {
    let (rpc_url, adapter_name) = match adapter {
        BridgeAdapter::Hyperlane => (TESTNET_RPC_URL, "Hyperlane (testnet)"),
        BridgeAdapter::Wormhole => (DEVNET_RPC_URL, "Wormhole (devnet)"),
    };

    println!("Using adapter: {}", adapter_name);

    let rpc_client = RpcClient::new(rpc_url);
    let payer = load_keypair()?;

    let signature = match adapter {
        BridgeAdapter::Hyperlane => {
            send_index_via_hyperlane(&rpc_client, &payer, destination_chain_id)?
        }
        BridgeAdapter::Wormhole => {
            send_index_via_wormhole(&rpc_client, &payer, destination_chain_id)?
        }
    };

    println!("Signature: {}", signature);

    Ok(())
}

fn load_keypair() -> Result<Keypair> {
    let key_path = format!("{}/.config/solana/id.json", std::env::var("HOME")?);
    Keypair::read_from_file(&key_path).map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))
}

fn send_index_via_hyperlane(
    rpc_client: &RpcClient,
    payer: &Keypair,
    destination_chain_id: u32,
) -> Result<solana_sdk::signature::Signature> {
    let portal_global = pda!(&[GLOBAL_SEED], &portal::ID);
    let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);

    // Build the SendIndex instruction with discriminator
    let mut instruction_data = calculate_instruction_discriminator("send_index").to_vec();
    instruction_data.extend_from_slice(&destination_chain_id.to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(portal_global, false),
        AccountMeta::new_readonly(portal_authority, false),
        AccountMeta::new_readonly(hyperlane_adapter::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let data_hyp = rpc_client.get_account_data(&pda!(&[b"global"], &hyperlane_adapter::ID))?;
    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;

    let hyp_user = rpc_client.get_account_data(&pda!(
        &[GLOBAL_SEED, DASH_SEED, payer.pubkey().as_ref()],
        &hyperlane_adapter::ID
    ));
    let user_global = match hyp_user {
        Ok(data) => Some(HyperlaneUserGlobal::try_deserialize(&mut data.as_slice())?),
        Err(_) => None,
    };

    // Remaining accounts for Hyperlane
    let hyperlane_accounts =
        HyperlaneRemainingAccounts::new(&payer.pubkey(), &global_hp, user_global.as_ref(), true);

    accounts.extend(hyperlane_accounts.to_account_metas());

    let instruction = SolanaInstruction {
        program_id: portal::ID,
        accounts,
        data: instruction_data,
    };

    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(600_000);

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[compute_budget_ix, instruction],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send transaction")?;

    Ok(signature)
}

fn send_index_via_wormhole(
    rpc_client: &RpcClient,
    payer: &Keypair,
    destination_chain_id: u32,
) -> Result<solana_sdk::signature::Signature> {
    let portal_global = pda!(&[GLOBAL_SEED], &portal::ID);
    let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);

    // Build the SendIndex instruction with discriminator
    let mut instruction_data = calculate_instruction_discriminator("send_index").to_vec();
    instruction_data.extend_from_slice(&destination_chain_id.to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(portal_global, false),
        AccountMeta::new_readonly(portal_authority, false),
        AccountMeta::new_readonly(wormhole_adapter::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let wormhole_accounts = WormholeRemainingAccounts::account_metas(true);
    accounts.extend(wormhole_accounts);

    let send_index_ix = SolanaInstruction {
        program_id: portal::ID,
        accounts,
        data: instruction_data,
    };

    // Build the relay instruction
    let peer_portal: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 107, 42, 123, 250, 95, 28, 3, 235, 250, 231, 121, 223,
        105, 136, 184, 172, 20, 202, 65, 85,
    ];

    let current_sequence =
        get_current_sequence_blocking(rpc_client, true).expect("Failed to get current sequence");

    println!("Requesting relay for sequence {}", current_sequence);

    let relay_ix = build_relay_instruction(
        &payer.pubkey(),
        get_wormhole_chain_id(destination_chain_id).unwrap(),
        current_sequence,
        &peer_portal,
        None,
        None,
    )?;

    // Build transaction with both instructions
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[compute_budget_ix, send_index_ix, relay_ix],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send transaction")?;

    Ok(signature)
}
