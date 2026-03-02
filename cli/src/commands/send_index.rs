use anchor_lang::system_program;
use anyhow::Result;
use m0_portal_common::{
    hyperlane_adapter, pda,
    portal::{self, constants::GLOBAL_SEED},
    wormhole_adapter, AUTHORITY_SEED,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::signer::Signer;

use crate::{types::calculate_instruction_discriminator, BridgeAdapter, Network};

use super::common::{get_rpc_config, load_keypair, send_via_hyperlane, send_via_wormhole};

pub async fn send_index(
    destination_chain_id: u32,
    adapter: BridgeAdapter,
    network: Network,
) -> Result<()> {
    let (rpc_url, adapter_name) = get_rpc_config(adapter, network);

    println!("Using adapter: {}", adapter_name);

    let rpc_client = RpcClient::new(rpc_url.to_string());
    let payer = load_keypair()?;

    let portal_global = pda!(&[GLOBAL_SEED], &portal::ID);
    let portal_authority = pda!(&[AUTHORITY_SEED], &portal::ID);

    // Build the SendIndex instruction data
    let mut instruction_data = calculate_instruction_discriminator("send_index").to_vec();
    instruction_data.extend_from_slice(&destination_chain_id.to_le_bytes());

    // Build base accounts (adapter-specific account will be set below)
    let adapter_id = match adapter {
        BridgeAdapter::Hyperlane => hyperlane_adapter::ID,
        BridgeAdapter::Wormhole => wormhole_adapter::ID,
    };

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(portal_global, false),
        AccountMeta::new_readonly(portal_authority, false),
        AccountMeta::new_readonly(adapter_id, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let signature = match adapter {
        BridgeAdapter::Hyperlane => {
            send_via_hyperlane(&rpc_client, &payer, accounts, instruction_data, true).await?
        }
        BridgeAdapter::Wormhole => {
            send_via_wormhole(
                &rpc_client,
                &payer,
                accounts,
                instruction_data,
                destination_chain_id,
                network == Network::Devnet,
            )
            .await?
        }
    };

    println!("Signature: {}", signature);

    Ok(())
}
