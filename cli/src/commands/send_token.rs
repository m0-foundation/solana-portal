use anchor_lang::system_program;
use anyhow::Result;
use m0_portal_common::{
    ext_swap, hyperlane_adapter, pda,
    portal::{
        self,
        constants::{CHAIN_PATHS_SEED, GLOBAL_SEED, MINT_AUTHORITY_SEED, M_VAULT_SEED},
    },
    wormhole_adapter, AUTHORITY_SEED,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use std::str::FromStr;

use crate::{types::calculate_instruction_discriminator, BridgeAdapter};

use super::common::{
    get_associated_token_address, get_rpc_config, load_keypair, parse_recipient,
    send_via_hyperlane, send_via_wormhole, EXTENSION_MINT, EXTENSION_PROGRAM, M_MINT,
    TOKEN_2022_PROGRAM_ID,
};

pub async fn send_token(
    amount: u64,
    destination_chain_id: u32,
    recipient: String,
    adapter: BridgeAdapter,
) -> Result<()> {
    let (rpc_url, adapter_name) = get_rpc_config(adapter);

    println!("Using adapter: {}", adapter_name);
    println!(
        "Sending {} tokens to chain {}",
        amount, destination_chain_id
    );

    let rpc_client = RpcClient::new(rpc_url.to_string());
    let payer = load_keypair()?;

    // Parse recipient as either Pubkey or hex bytes
    let recipient_bytes = parse_recipient(&recipient)?;
    let destination_token = parse_recipient(&"0x437cc33344a0B27A429f795ff6B469C72698B291")?;
    println!("Recipient: 0x{}", hex::encode(recipient_bytes));

    // Parse token addresses
    let m_mint = Pubkey::from_str(M_MINT)?;
    let extension_mint = Pubkey::from_str(EXTENSION_MINT)?;
    let extension_program = Pubkey::from_str(EXTENSION_PROGRAM)?;
    let token_2022_program = Pubkey::from_str(TOKEN_2022_PROGRAM_ID)?;

    // Derive PDAs
    let portal_global = pda!(&[GLOBAL_SEED], &portal::ID);
    let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);
    let swap_global = pda!(&[GLOBAL_SEED], &ext_swap::ID);
    let extension_global = pda!(&[GLOBAL_SEED], &extension_program);
    let ext_m_vault_auth = pda!(&[M_VAULT_SEED], &extension_program);
    let ext_mint_authority = pda!(&[MINT_AUTHORITY_SEED], &extension_program);
    let chain_paths = pda!(
        &[CHAIN_PATHS_SEED, &destination_chain_id.to_le_bytes()],
        &portal::ID
    );

    // Get token accounts
    let m_token_account =
        get_associated_token_address(&portal_authority, &m_mint, &token_2022_program);
    let extension_token_account =
        get_associated_token_address(&payer.pubkey(), &extension_mint, &token_2022_program);
    let ext_m_vault = get_associated_token_address(&ext_m_vault_auth, &m_mint, &token_2022_program);

    // Build the SendToken instruction data
    let mut instruction_data = calculate_instruction_discriminator("send_token").to_vec();
    instruction_data.extend_from_slice(&amount.to_le_bytes());
    instruction_data.extend_from_slice(&destination_token);
    instruction_data.extend_from_slice(&destination_chain_id.to_le_bytes());
    instruction_data.extend_from_slice(&recipient_bytes);

    // Build base accounts
    let adapter_id = match adapter {
        BridgeAdapter::Hyperlane => hyperlane_adapter::ID,
        BridgeAdapter::Wormhole => wormhole_adapter::ID,
    };

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(portal_global, false),
        AccountMeta::new_readonly(swap_global, false),
        AccountMeta::new_readonly(chain_paths, false),
        AccountMeta::new(extension_global, false),
        AccountMeta::new(m_mint, false),
        AccountMeta::new(extension_mint, false),
        AccountMeta::new(m_token_account, false),
        AccountMeta::new(extension_token_account, false),
        AccountMeta::new_readonly(portal_authority, false),
        AccountMeta::new(ext_m_vault, false),
        AccountMeta::new_readonly(ext_m_vault_auth, false),
        AccountMeta::new_readonly(ext_mint_authority, false),
        AccountMeta::new_readonly(ext_swap::ID, false),
        AccountMeta::new_readonly(extension_program, false),
        AccountMeta::new_readonly(token_2022_program, false),
        AccountMeta::new_readonly(token_2022_program, false),
        AccountMeta::new_readonly(adapter_id, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let signature = match adapter {
        BridgeAdapter::Hyperlane => {
            send_via_hyperlane(&rpc_client, &payer, accounts, instruction_data, false).await?
        }
        BridgeAdapter::Wormhole => {
            send_via_wormhole(
                &rpc_client,
                &payer,
                accounts,
                instruction_data,
                destination_chain_id,
            )
            .await?
        }
    };

    println!("Signature: {}", signature);

    Ok(())
}
