use crate::BridgeAdapter;
use anchor_lang::AccountDeserialize;
use anyhow::{Context, Result};
use m0_portal_common::{
    build_relay_instruction, get_current_sequence, get_wormhole_chain_id,
    hyperlane_adapter::{
        self,
        accounts::{HyperlaneGlobal, HyperlaneUserGlobal},
        constants::DASH_SEED,
    },
    pda,
    portal::{self, constants::GLOBAL_SEED},
    wormhole_adapter::accounts::WormholeGlobal,
    HyperlaneRemainingAccounts, WormholeRemainingAccounts,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    address_lookup_table::state::AddressLookupTable,
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction as SolanaInstruction},
    message::{v0, AddressLookupTableAccount, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use std::str::FromStr;

// Token addresses (testnet/devnet)
pub const M_MINT: &str = "mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH";
pub const EXTENSION_MINT: &str = "mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp";
pub const EXTENSION_PROGRAM: &str = "wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko";
pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

// Wormhole peer portal address
pub const PEER_PORTAL: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 234, 174, 73, 107, 205, 169, 60, 204, 211, 253, 111, 246,
    9, 99, 71, 151, 158, 135, 177, 83,
];

/// Get RPC URL and adapter name for the given bridge adapter
pub fn get_rpc_config(adapter: BridgeAdapter) -> (String, &'static str) {
    match adapter {
        BridgeAdapter::Hyperlane => (get_testnet_rpc_url(), "Hyperlane (testnet)"),
        BridgeAdapter::Wormhole => (get_devnet_rpc_url(), "Wormhole (devnet)"),
    }
}

/// Get testnet RPC URL from TESTNET_RPC_URL env var, or use default
pub fn get_testnet_rpc_url() -> String {
    std::env::var("TESTNET_RPC_URL")
        .unwrap_or_else(|_| "https://api.testnet.solana.com".to_string())
}

/// Get devnet RPC URL from DEVNET_RPC_URL env var, or use default
pub fn get_devnet_rpc_url() -> String {
    std::env::var("DEVNET_RPC_URL").unwrap_or_else(|_| "https://api.devnet.solana.com".to_string())
}

/// Load keypair from the default Solana config location
pub fn load_keypair() -> Result<Keypair> {
    let key_path = format!("{}/.config/solana/id.json", std::env::var("HOME")?);
    Keypair::read_from_file(&key_path).map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))
}

/// Compute the associated token address for a given wallet and mint
pub fn get_associated_token_address(
    wallet: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Pubkey {
    let associated_token_program = Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID).unwrap();
    let (address, _) = Pubkey::find_program_address(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        &associated_token_program,
    );
    address
}

/// Parse recipient as either a Solana Pubkey or 32-byte hex string
pub fn parse_recipient(recipient: &str) -> Result<[u8; 32]> {
    // Try parsing as Solana pubkey first
    if let Ok(pubkey) = Pubkey::from_str(recipient) {
        return Ok(pubkey.to_bytes());
    }

    // Try parsing as hex (with or without 0x prefix)
    let hex_str = recipient.strip_prefix("0x").unwrap_or(recipient);
    let bytes =
        hex::decode(hex_str).context("Invalid recipient format (not a valid pubkey or hex)")?;

    if bytes.len() == 32 {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    } else if bytes.len() == 20 {
        // EVM address
        let mut arr = [0u8; 32];
        arr[12..].copy_from_slice(&bytes);
        Ok(arr)
    } else {
        anyhow::bail!("Invalid recipient length: expected 32 bytes (Solana) or 20 bytes (EVM)")
    }
}

/// Fetch Hyperlane global and user global accounts, return remaining accounts
pub async fn get_hyperlane_remaining_accounts(
    rpc_client: &RpcClient,
    payer: &Pubkey,
    include_igp: bool,
) -> Result<HyperlaneRemainingAccounts> {
    let data_hyp = rpc_client
        .get_account_data(&pda!(&[b"global"], &hyperlane_adapter::ID))
        .await?;

    let global_hp = HyperlaneGlobal::try_deserialize(&mut data_hyp.as_slice())?;

    let hyp_user = rpc_client
        .get_account_data(&pda!(
            &[GLOBAL_SEED, DASH_SEED, payer.as_ref()],
            &hyperlane_adapter::ID
        ))
        .await;

    let user_global = match hyp_user {
        Ok(data) => Some(HyperlaneUserGlobal::try_deserialize(&mut data.as_slice())?),
        Err(_) => None,
    };

    Ok(HyperlaneRemainingAccounts::new(
        payer,
        &global_hp,
        user_global.as_ref(),
        include_igp,
    ))
}

/// Send a transaction via Hyperlane adapter
pub async fn send_via_hyperlane(
    rpc_client: &RpcClient,
    payer: &Keypair,
    accounts: Vec<AccountMeta>,
    instruction_data: Vec<u8>,
    include_igp: bool,
) -> Result<solana_sdk::signature::Signature> {
    let mut all_accounts = accounts;

    // Get and append Hyperlane remaining accounts
    let hyperlane_accounts =
        get_hyperlane_remaining_accounts(rpc_client, &payer.pubkey(), include_igp).await?;
    all_accounts.extend(hyperlane_accounts.to_account_metas());

    let instruction = SolanaInstruction {
        program_id: portal::ID,
        accounts: all_accounts,
        data: instruction_data,
    };

    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(600_000);

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[compute_budget_ix, instruction],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .context("Failed to send transaction")?;

    Ok(signature)
}

/// Send a transaction via Wormhole adapter with relay instruction
pub async fn send_via_wormhole(
    rpc_client: &RpcClient,
    payer: &Keypair,
    accounts: Vec<AccountMeta>,
    instruction_data: Vec<u8>,
    destination_chain_id: u32,
) -> Result<solana_sdk::signature::Signature> {
    let mut all_accounts = accounts;

    // Get and append Wormhole remaining accounts
    let wormhole_accounts = WormholeRemainingAccounts::account_metas(true);
    all_accounts.extend(wormhole_accounts);

    let send_ix = SolanaInstruction {
        program_id: portal::ID,
        accounts: all_accounts,
        data: instruction_data,
    };

    // Build the relay instruction
    let current_sequence = get_current_sequence(rpc_client, true)
        .await
        .expect("Failed to get current sequence");

    println!("Requesting relay for sequence {}", current_sequence);

    let relay_ix = build_relay_instruction(
        &payer.pubkey(),
        get_wormhole_chain_id(destination_chain_id).unwrap(),
        current_sequence,
        &PEER_PORTAL,
        Some(350_000),
        Some(25_000_000),
    )
    .await?;

    // Build transaction with both instructions
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(500_000);

    let transaction = build_versioned_tx_with_lut(
        rpc_client,
        vec![compute_budget_ix, send_ix, relay_ix],
        payer,
    )
    .await?;

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .context("Failed to send transaction")?;

    Ok(signature)
}

pub async fn build_versioned_tx_with_lut(
    rpc: &RpcClient,
    instructions: Vec<solana_sdk::instruction::Instruction>,
    signer: &Keypair,
) -> Result<VersionedTransaction> {
    let data_wh = rpc
        .get_account_data(&pda!(&[GLOBAL_SEED], &wormhole_adapter::ID))
        .await?;
    let global_wh = WormholeGlobal::try_deserialize(&mut data_wh.as_slice())?;
    let lut = global_wh
        .receive_lut
        .expect("expected receive LUT to be initialized");

    let recent_blockhash = rpc.get_latest_blockhash().await?;

    let lut_account = rpc.get_account(&lut).await?;
    let address_lookup_table = AddressLookupTableAccount {
        key: lut,
        addresses: AddressLookupTable::deserialize(&lut_account.data)?
            .addresses
            .to_vec(),
    };

    let swap_lut_account =
        Pubkey::from_str("6GhuWPuAmiJeeSVsr58KjqHcAejJRndCx9BVtHkaYHUR").unwrap();

    let swap_lut = AddressLookupTableAccount {
        key: swap_lut_account,
        addresses: AddressLookupTable::deserialize(
            &rpc.get_account(&swap_lut_account).await?.data,
        )?
        .addresses
        .to_vec(),
    };

    let message = v0::Message::try_compile(
        &signer.pubkey(),
        &instructions,
        &[address_lookup_table, swap_lut],
        recent_blockhash,
    )?;

    let versioned_message = VersionedMessage::V0(message);
    Ok(VersionedTransaction::try_new(versioned_message, &[signer])?)
}
