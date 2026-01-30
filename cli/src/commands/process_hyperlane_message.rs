use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use m0_portal_common::{
    hyperlane_adapter::{self, constants::PAYER_SEED},
    pda,
    portal::{
        self,
        constants::{GLOBAL_SEED, MESSAGE_SEED},
    },
    AUTHORITY_SEED,
};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcSimulateTransactionConfig};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

const DEFAULT_RPC_URL: &str = "https://api.testnet.solana.com";

/// InboxProcess instruction data
#[derive(BorshSerialize, BorshDeserialize)]
pub struct InboxProcess {
    pub metadata: Vec<u8>,
    pub message: Vec<u8>,
}

/// Mailbox instruction discriminator for InboxProcess
const INBOX_PROCESS_DISCRIMINATOR: u8 = 1;

/// Process a Hyperlane mailbox message
/// Reference: https://github.com/hyperlane-xyz/hyperlane-monorepo/tree/main/rust/sealevel/programs/mailbox
pub fn process_hyperlane_message(message_id: String, raw: String) -> Result<()> {
    println!("Processing Hyperlane message {}...", message_id);

    let payer = load_keypair()?;
    let rpc_client = RpcClient::new_with_commitment(DEFAULT_RPC_URL, CommitmentConfig::confirmed());

    let message_id_bytes = hex::decode(&message_id.trim_start_matches("0x"))?;
    let raw_bytes = hex::decode(raw.trim_start_matches("0x")).unwrap();
    let recipient = hyperlane_adapter::ID;

    let inbox_process = InboxProcess {
        // TODO: populate metadata
        // https://github.com/hyperlane-xyz/hyperlane-monorepo/blob/main/rust/sealevel/programs/mailbox/src/instruction.rs#L92
        // https://github.com/hyperlane-xyz/hyperlane-monorepo/blob/b7cdfc5d3373572dd4dc675c45f0d6f07a4b374c/rust/sealevel/programs/ism/multisig-ism-message-id/src/metadata.rs#L7
        metadata: vec![],
        message: raw_bytes,
    };

    // Serialize instruction data: discriminator (u8) + borsh-serialized InboxProcess
    let mut instruction_data = vec![INBOX_PROCESS_DISCRIMINATOR];
    instruction_data.extend_from_slice(&inbox_process.try_to_vec()?);

    let mailbox_program_id =
        Pubkey::from_str("75HBBLae3ddeneJVrZeyrDfv6vb7SMC3aCpBucSXS5aR").unwrap();

    // Derive Inbox PDA
    let (inbox_pda, _) =
        Pubkey::find_program_address(&[b"hyperlane", b"-", b"inbox"], &mailbox_program_id);

    // Derive mailbox process authority for the recipient
    let (mailbox_process_authority, _) = Pubkey::find_program_address(
        &[
            b"hyperlane",
            b"-",
            b"process_authority",
            b"-",
            recipient.as_ref(),
        ],
        &mailbox_program_id,
    );

    // Derive processed message PDA
    let (processed_message_pda, _) = Pubkey::find_program_address(
        &[
            b"hyperlane",
            b"-",
            b"processed_message",
            b"-",
            &message_id_bytes,
        ],
        &mailbox_program_id,
    );

    // Basic accounts needed for processing
    let mut accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(inbox_pda, false),
        AccountMeta::new_readonly(mailbox_process_authority, false),
        AccountMeta::new(processed_message_pda, false),
    ];

    // ISM
    accounts.extend([
        AccountMeta::new_readonly(pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID), false),
        AccountMeta::new_readonly(
            Pubkey::from_str("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV").unwrap(),
            false,
        ),
        AccountMeta::new_readonly(
            Pubkey::from_str("4GHxwWyKB9exhKG4fdyU2hfLgfFzhHp2WcsSKc2uNR1k").unwrap(),
            false,
        ),
    ]);

    let (ism_meta, _) = Pubkey::find_program_address(
        &[
            b"multisig_ism_message_id",
            b"-",
            &11155111u32.to_le_bytes(),
            b"-",
            b"domain_data",
        ],
        &Pubkey::from_str("4GHxwWyKB9exhKG4fdyU2hfLgfFzhHp2WcsSKc2uNR1k").unwrap(),
    );

    // Get the account metas required for the ISM verify instruction.
    accounts.extend(vec![AccountMeta::new_readonly(ism_meta, false)]);

    // The recipient
    accounts.extend([AccountMeta::new_readonly(recipient, false)]);

    // Get account metas required for the Handle instruction
    let hyperlane_adapter_authority = pda!(&[AUTHORITY_SEED], &hyperlane_adapter::ID);
    let hyperlane_global = pda!(&[GLOBAL_SEED], &hyperlane_adapter::ID);
    let portal_global = pda!(&[GLOBAL_SEED], &portal::ID);
    let payer_account = pda!(&[PAYER_SEED], &hyperlane_adapter::ID);
    let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);
    let message_account = pda!(&[MESSAGE_SEED, &message_id_bytes], &portal::ID);

    // Accounts needed by all payload types
    accounts.extend(vec![
        AccountMeta::new(hyperlane_adapter_authority, false),
        AccountMeta::new(payer_account, false),
        AccountMeta::new_readonly(hyperlane_global, false),
        AccountMeta::new(portal_global, false),
        AccountMeta::new_readonly(portal_authority, false),
        AccountMeta::new(message_account, false),
        AccountMeta::new_readonly(portal::ID, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ]);

    let instruction = Instruction {
        program_id: mailbox_program_id,
        accounts,
        data: instruction_data,
    };

    // Simulate
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let result = rpc_client.simulate_transaction_with_config(
        &transaction,
        RpcSimulateTransactionConfig {
            sig_verify: false,
            replace_recent_blockhash: true,
            ..Default::default()
        },
    );

    println!("Simulation result: {:#?}", result.unwrap().value);

    Ok(())
}

/// Load keypair from default Solana config
fn load_keypair() -> Result<Keypair> {
    let key_path = format!("{}/.config/solana/id.json", std::env::var("HOME")?);
    Keypair::read_from_file(&key_path).map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))
}
