use anchor_lang::{pubkey, system_program, AccountDeserialize};
use anyhow::{Context, Result};
use common::{
    hyperlane_adapter::{
        self,
        accounts::{HyperlaneGlobal, HyperlaneUserGlobal},
        constants::{
            DASH_SEED, DISPATCHED_MESSAGE_SEED, HYPERLANE_SEED, OUTBOX_SEED, UNIQUE_MESSAGE_SEED,
        },
    },
    pda,
    portal::{self, constants::GLOBAL_SEED},
    HyperlaneRemainingAccounts, AUTHORITY_SEED,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction as SolanaInstruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    transaction::Transaction,
};

// Testnet values
const RPC_URL: &str = "https://api.testnet.solana.com";
const MAILBOX_PROGRAM_ID: Pubkey = pubkey!("75HBBLae3ddeneJVrZeyrDfv6vb7SMC3aCpBucSXS5aR");

pub fn send_index(destination_chain_id: u32) -> Result<()> {
    let rpc_client = RpcClient::new(RPC_URL);
    let payer = load_keypair()?;

    let signature = send_index_transaction(&rpc_client, &payer, destination_chain_id)?;
    println!("Signature: {}", signature);

    Ok(())
}

fn load_keypair() -> Result<Keypair> {
    let key_path = format!("{}/.config/solana/id.json", std::env::var("HOME")?);
    Keypair::read_from_file(&key_path).map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))
}

fn send_index_transaction(
    rpc_client: &RpcClient,
    payer: &Keypair,
    destination_chain_id: u32,
) -> Result<solana_sdk::signature::Signature> {
    let portal_global = pda!(&[GLOBAL_SEED], &portal::ID);
    let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);

    // Build the SendIndex instruction with discriminator
    let mut instruction_data = vec![92, 203, 229, 128, 118, 111, 243, 53];
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
        &[GLOBAL_SEED, payer.pubkey().as_ref()],
        &hyperlane_adapter::ID
    ));
    let user_global = match hyp_user {
        Ok(data) => Some(HyperlaneUserGlobal::try_deserialize(&mut data.as_slice())?),
        Err(_) => None,
    };

    // Unique message PDA based on user global nonce
    let unique_message = pda!(
        &[
            UNIQUE_MESSAGE_SEED,
            &user_global
                .map(|g| g.nonce)
                .unwrap_or_default()
                .to_be_bytes()
        ],
        &hyperlane_adapter::ID
    );

    // Remaining accounts for Hyperlane
    let mut hyperlane_accounts =
        HyperlaneRemainingAccounts::new(&payer.pubkey(), &global_hp, user_global.as_ref());

    // Update mailbox accounts (program_id is different on testnet)
    hyperlane_accounts.mailbox_program = MAILBOX_PROGRAM_ID;
    hyperlane_accounts.mailbox_outbox = pda!(
        &[HYPERLANE_SEED, DASH_SEED, OUTBOX_SEED],
        &MAILBOX_PROGRAM_ID
    );
    hyperlane_accounts.dispatched_message = pda!(
        &[
            HYPERLANE_SEED,
            DASH_SEED,
            DISPATCHED_MESSAGE_SEED,
            DASH_SEED,
            unique_message.as_ref(),
        ],
        &MAILBOX_PROGRAM_ID
    );

    accounts.extend(hyperlane_accounts.to_account_metas());

    let instruction = SolanaInstruction {
        program_id: portal::ID,
        accounts,
        data: instruction_data,
    };

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .context("Failed to send transaction")?;

    Ok(signature)
}
