use anchor_lang::AccountDeserialize;
use anyhow::{Context, Result};
use m0_portal_common::{
    earn, ext_swap,
    hyperlane_adapter::{
        self,
        constants::{
            DASH_SEED, DISPATCH_SEED_1, DISPATCH_SEED_2, GLOBAL_SEED as HL_GLOBAL_SEED,
            HYPERLANE_IGP_SEED, HYPERLANE_SEED, METADATA_SEED_1, METADATA_SEED_2, METADATA_SEED_3,
            OUTBOX_SEED, PAYER_SEED, PROCESS_AUTHORITY, PROGRAM_DATA_SEED,
        },
    },
    pda, portal, AUTHORITY_SEED,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    address_lookup_table::instruction::{create_lookup_table, extend_lookup_table},
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    system_program::ID as SYSTEM_PROGRAM_ID,
    transaction::Transaction,
};
use std::str::FromStr;

const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const SPL_NOOP_PROGRAM_ID: &str = "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV";
const MAINNET_RPC_URL: &str = "https://hatty-73mn84-fast-mainnet.helius-rpc.com";
const SWAP_GLOBAL: &str = "6U4ZZZkftbuHxjRDHUfh83M9zG66aAAXDV3xTRX7yePr";

struct HyperlaneConfig {
    mailbox: Pubkey,
    igp_program: Pubkey,
    igp_account: Pubkey,
    igp_overhead_account: Pubkey,
}

impl HyperlaneConfig {
    fn mainnet() -> Self {
        Self {
            mailbox: Pubkey::from_str("E588QtVUvresuXq2KoNEwAmoifCzYGpRBdHByN9KQMbi").unwrap(),
            igp_program: Pubkey::from_str("BhNcatUDC2D5JTyeaqrdSukiVFsEHK7e3hVmKMztwefv").unwrap(),
            igp_account: Pubkey::from_str("JAvHW21tYXE9dtdG83DReqU2b4LUexFuCbtJT5tF8X6M").unwrap(),
            igp_overhead_account: Pubkey::from_str("AkeHBbE5JkwVppujCQQ6WuxsVsJtruBAjUo6fDCFp6fF")
                .unwrap(),
        }
    }

    fn testnet() -> Self {
        Self {
            mailbox: Pubkey::from_str("75HBBLae3ddeneJVrZeyrDfv6vb7SMC3aCpBucSXS5aR").unwrap(),
            igp_program: Pubkey::from_str("5p7Hii6CJL4xGBYYTGEQmH9LnUSZteFJUu9AVLDExZX2").unwrap(),
            igp_account: Pubkey::from_str("9SQVtTNsbipdMzumhzi6X8GwojiSMwBfqAhS7FgyTcqy").unwrap(),
            igp_overhead_account: Pubkey::from_str("hBHAApi5ZoeCYHqDdCKkCzVKmBdwywdT3hMqe327eZB")
                .unwrap(),
        }
    }
}

fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    let associated_token_program = Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID).unwrap();
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &associated_token_program,
    )
    .0
}

async fn build_lut_addresses(network: &str) -> Result<Vec<Pubkey>> {
    let m_mint = Pubkey::from_str("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH").unwrap();

    let hyperlane = if network == "mainnet" {
        HyperlaneConfig::mainnet()
    } else {
        HyperlaneConfig::testnet()
    };

    let token_2022_program = Pubkey::from_str(TOKEN_2022_PROGRAM_ID).unwrap();
    let associated_token_program = Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID).unwrap();
    let spl_noop_program = Pubkey::from_str(SPL_NOOP_PROGRAM_ID).unwrap();
    let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);

    let mut addresses = vec![
        // Program IDs
        hyperlane_adapter::ID,
        portal::ID,
        earn::ID,
        ext_swap::ID,
        token_2022_program,
        associated_token_program,
        SYSTEM_PROGRAM_ID,
        spl_noop_program,
        // Hyperlane external programs
        hyperlane.mailbox,
        hyperlane.igp_program,
        // Global state PDAs
        pda!(&[HL_GLOBAL_SEED], &hyperlane_adapter::ID),
        pda!(&[portal::constants::GLOBAL_SEED], &portal::ID),
        pda!(&[earn::constants::GLOBAL_SEED], &earn::ID),
        pda!(&[ext_swap::constants::GLOBAL_SEED], &ext_swap::ID),
        // Authority PDAs
        pda!(&[AUTHORITY_SEED], &hyperlane_adapter::ID),
        portal_authority,
        // Hyperlane-specific PDAs
        pda!(&[PAYER_SEED], &hyperlane_adapter::ID),
        pda!(
            &[DISPATCH_SEED_1, DASH_SEED, DISPATCH_SEED_2],
            &hyperlane_adapter::ID
        ),
        pda!(
            &[HYPERLANE_SEED, DASH_SEED, OUTBOX_SEED],
            &hyperlane.mailbox
        ),
        pda!(
            &[
                HYPERLANE_SEED,
                DASH_SEED,
                PROCESS_AUTHORITY,
                DASH_SEED,
                hyperlane_adapter::ID.as_ref()
            ],
            &hyperlane.mailbox
        ),
        pda!(
            &[HYPERLANE_IGP_SEED, DASH_SEED, PROGRAM_DATA_SEED],
            &hyperlane.igp_program
        ),
        pda!(
            &[
                METADATA_SEED_1,
                DASH_SEED,
                METADATA_SEED_2,
                DASH_SEED,
                METADATA_SEED_3
            ],
            &hyperlane_adapter::ID
        ),
        // Hyperlane IGP accounts
        hyperlane.igp_account,
        hyperlane.igp_overhead_account,
        // Token accounts
        m_mint,
        get_associated_token_address(&portal_authority, &m_mint, &token_2022_program),
    ];

    // Fetch whitelisted extensions from swap facility (always from mainnet)
    let mainnet_client =
        RpcClient::new_with_commitment(MAINNET_RPC_URL.to_string(), CommitmentConfig::confirmed());
    let swap_facility_pk = Pubkey::from_str(SWAP_GLOBAL).unwrap();
    let swap_data = mainnet_client
        .get_account_data(&swap_facility_pk)
        .await
        .context("Failed to fetch swap facility account")?;
    let swap_acc = ext_swap::accounts::SwapGlobal::try_deserialize(&mut swap_data.as_slice())
        .map_err(|e| anyhow::anyhow!("Failed to deserialize swap facility: {}", e))?;

    for ext in &swap_acc.whitelisted_extensions {
        addresses.push(ext.program_id);
        addresses.push(ext.mint);
    }

    Ok(addresses)
}

pub async fn create_hyperlane_lut(network: String) -> Result<()> {
    let rpc_url = match network.as_str() {
        "mainnet" => MAINNET_RPC_URL,
        "testnet" => "https://api.testnet.solana.com",
        _ => anyhow::bail!("Invalid network: must be 'mainnet' or 'devnet'"),
    };

    let client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    let key_path = format!(
        "{}/.config/solana/id.json",
        std::env::var("HOME").expect("HOME env var not set")
    );
    let payer = Keypair::read_from_file(&key_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;

    let addresses = build_lut_addresses(&network).await?;

    // Get a recent slot for LUT derivation
    let recent_slot = client.get_slot().await?;

    // Create LUT
    let (create_ix, lut_address) = create_lookup_table(payer.pubkey(), payer.pubkey(), recent_slot);
    println!("Creating LUT: {}", lut_address);

    // Split extend into batches of 20 to stay within tx size limits
    let batches: Vec<Vec<Pubkey>> = addresses.chunks(20).map(|c| c.to_vec()).collect();

    // Transaction 1: Create LUT + first batch of extends
    let recent_blockhash = client.get_latest_blockhash().await?;

    let extend_ix = extend_lookup_table(
        lut_address,
        payer.pubkey(),
        Some(payer.pubkey()),
        batches[0].clone(),
    );

    let tx = Transaction::new_signed_with_payer(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(300_000),
            create_ix,
            extend_ix,
        ],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let sig = client
        .send_and_confirm_transaction(&tx)
        .await
        .context("Failed to create LUT")?;
    println!("  tx: {}", sig);

    // Remaining batches (if any)
    for (i, batch) in batches.iter().enumerate().skip(1) {
        println!("Extending batch {}/{}...", i + 1, batches.len());
        let recent_blockhash = client.get_latest_blockhash().await?;

        let extend_ix = extend_lookup_table(
            lut_address,
            payer.pubkey(),
            Some(payer.pubkey()),
            batch.clone(),
        );

        let tx = Transaction::new_signed_with_payer(
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(200_000),
                extend_ix,
            ],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );

        let sig = client
            .send_and_confirm_transaction(&tx)
            .await
            .context("Failed to extend LUT")?;
        println!("  tx: {}", sig);
    }

    println!(
        "LUT created: {} ({} entries, {})",
        lut_address,
        addresses.len(),
        network
    );

    Ok(())
}
