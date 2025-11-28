use anyhow::{Ok, Result};
use base64::{prelude::BASE64_STANDARD, Engine};
use common::{pda, wormhole_adapter};
use solana_client::rpc_client::RpcClient;
use solana_rpc_client_types::config::RpcSimulateTransactionConfig;
use solana_sdk::{
    instruction::{AccountMeta, Instruction as SolanaInstruction},
    message::Message,
    pubkey::Pubkey,
    transaction::Transaction,
};

use crate::types::*;

const PAYER: Pubkey = Pubkey::from_str_const("D76ySoHPwD8U2nnTTDqXeUJQg5UkD9UD1PUE1rnvPAGm");

pub fn resolve_execute(tx_hash: String) -> Result<()> {
    let rpc_client = RpcClient::new("https://lindsy-gxe51w-fast-devnet.helius-rpc.com");

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

    let mut requested_accounts = vec![];

    // Known placeholders and accounts
    let result_pda = pda!(
        &[b"executor-account-resolver:result"],
        &wormhole_adapter::ID
    );
    let guardian_set = Pubkey::new_from_array(*b"guardian_set_0000000000000000000");

    // Run simulation loop up to 3 times
    for _ in 0..3 {
        let transaction =
            create_transaction(vaa_body.clone(), &rpc_client, requested_accounts.clone())?;

        // Simulate transaction
        let simulation_result = rpc_client.simulate_transaction_with_config(
            &transaction,
            RpcSimulateTransactionConfig {
                sig_verify: false,
                replace_recent_blockhash: true,
                ..RpcSimulateTransactionConfig::default()
            },
        )?;

        // Parse the return data
        let return_data = simulation_result.value.return_data.unwrap();
        let data_bytes = BASE64_STANDARD.decode(&return_data.data.0)?;

        // Look for the pattern: 01 (Missing variant)
        let mut missing_accounts = vec![];
        assert!(data_bytes[0] == 1);

        let accounts_len = u32::from_le_bytes(data_bytes[1..5].try_into()?) as usize;

        println!("=== Missing Accounts ===");

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
        requested_accounts.extend(
            missing_accounts
                .into_iter()
                .map(|pubkey| AccountMeta::new_readonly(pubkey, false)),
        );
    }

    Ok(())
}

fn create_transaction(
    vaa_body: Vec<u8>,
    rpc_client: &RpcClient,
    requested_accounts: Vec<AccountMeta>,
) -> Result<Transaction, anyhow::Error> {
    let mut instruction_data = Vec::new();
    instruction_data.extend_from_slice(&[148, 184, 169, 222, 207, 8, 154, 127]);
    instruction_data.extend_from_slice(&(vaa_body.len() as u32).to_le_bytes());
    instruction_data.extend_from_slice(&vaa_body);

    let mut accounts = vec![AccountMeta::new(PAYER, true)];
    accounts.extend(requested_accounts);

    let instruction = SolanaInstruction {
        program_id: wormhole_adapter::ID,
        accounts,
        data: instruction_data,
    };

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let message = Message::new_with_blockhash(&[instruction], None, &recent_blockhash);

    Ok(Transaction::new_unsigned(message))
}
