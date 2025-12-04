use anchor_lang::{pubkey, system_program, AccountDeserialize};
use anyhow::{Context, Result};
use common::{
    hyperlane_adapter::{
        self,
        accounts::{HyperlaneGlobal, HyperlaneUserGlobal},
        constants::{
            DASH_SEED, DISPATCHED_MESSAGE_SEED, DISPATCH_SEED_1, DISPATCH_SEED_2, GAS_PAYMENT_SEED,
            HYPERLANE_IGP_SEED, HYPERLANE_SEED, OUTBOX_SEED, PROGRAM_DATA_SEED, SPL_NOOP,
            UNIQUE_MESSAGE_SEED,
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
    ))?;
    let user_hp = HyperlaneUserGlobal::try_deserialize(&mut hyp_user.as_slice())?;

    let unique_message = pda!(
        &[UNIQUE_MESSAGE_SEED, user_hp.nonce.to_le_bytes().as_ref()],
        &hyperlane_adapter::ID
    );

    // Add Hyperlane remaining accounts (using testnet values)
    let hyperlane_accounts = HyperlaneRemainingAccounts {
        hyperlane_global: pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID),
        mailbox_outbox: pda!(
            &[HYPERLANE_SEED, DASH_SEED, OUTBOX_SEED],
            &MAILBOX_PROGRAM_ID
        ),
        dispatch_authority: pda!(
            &[DISPATCH_SEED_1, DASH_SEED, DISPATCH_SEED_2],
            &hyperlane_adapter::ID
        ),
        unique_message,
        dispatched_message: pda!(
            &[
                HYPERLANE_SEED,
                DASH_SEED,
                DISPATCHED_MESSAGE_SEED,
                DASH_SEED,
                unique_message.as_ref(),
            ],
            &MAILBOX_PROGRAM_ID
        ),
        igp_program_id: global_hp.igp_program_id,
        igp_program_data: pda!(
            &[HYPERLANE_IGP_SEED, DASH_SEED, PROGRAM_DATA_SEED],
            &global_hp.igp_program_id
        ),
        igp_gas_payment: pda!(
            &[
                HYPERLANE_IGP_SEED,
                DASH_SEED,
                GAS_PAYMENT_SEED,
                DASH_SEED,
                unique_message.as_ref()
            ],
            &global_hp.igp_program_id
        ),
        igp_account: global_hp.igp_account,
        igp_overhead_account: global_hp.igp_overhead_account,
        mailbox_program: MAILBOX_PROGRAM_ID,
        spl_noop_program: SPL_NOOP,
    };
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
