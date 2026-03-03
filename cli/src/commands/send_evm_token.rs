use alloy::{
    primitives::{Address, FixedBytes, U256},
    providers::Provider,
    rpc::types::TransactionRequest,
    sol_types::{SolCall, SolValue},
};
use anyhow::{Context, Result};

use crate::{
    commands::common::parse_recipient,
    types::evm::{address_to_bytes32, Erc20, Portal, PAYLOAD_TYPE_TOKEN_TRANSFER},
    BridgeAdapter, Network,
};

use super::evm_common::{
    create_provider, get_adapter_address, get_adapter_args_and_value, get_adapter_name,
    get_portal_address, load_private_key, send_and_confirm_transaction, NetworkConfig,
};

/// Send token transfer from EVM to Solana via the Portal contract
pub async fn send_evm_token(
    amount: u128,
    recipient: String,
    adapter: BridgeAdapter,
    network: Network,
) -> Result<()> {
    let config = NetworkConfig::from_network(network)?;
    println!("Network: {}", config.network_label);
    println!("Using adapter: {}", get_adapter_name(adapter));
    println!("Sending {} tokens to Solana", amount);

    // Load private key and create provider
    let signer = load_private_key()
        .context("Failed to load private key. Make sure PRIVATE_KEY env var is set")?;
    let sender_address = signer.address();
    let provider = create_provider(signer, &config)?;

    // Get addresses
    let contract_address = get_portal_address()?;
    let adapter_address = get_adapter_address(adapter)?;
    let refund_address = address_to_bytes32(sender_address);

    let source_token_address: Address = "0x866A2BF4E572CbcF37D5071A7a58503Bfb36be1b"
        .parse()
        .context("Invalid source token address")?;
    let destination_token_bytes = parse_recipient(&"mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp")?;

    // Parse recipient (Solana address as bytes32)
    let recipient_bytes = parse_recipient(&recipient)?;
    println!("Recipient: 0x{}", hex::encode(recipient_bytes));

    // Check allowance and approve if needed
    let amount_u256 = U256::from(amount);
    let allowance_call = Erc20::allowanceCall {
        owner: sender_address,
        spender: contract_address,
    };
    let allowance_tx = TransactionRequest::default()
        .to(source_token_address)
        .input(allowance_call.abi_encode().into());
    let allowance_result = provider
        .call(&allowance_tx)
        .await
        .context("Failed to check token allowance")?;
    let current_allowance =
        U256::abi_decode(&allowance_result, true).context("Failed to decode allowance")?;

    if current_allowance < amount_u256 {
        println!("Approving Portal contract to spend {} tokens...", amount);
        let approve_call = Erc20::approveCall {
            spender: contract_address,
            amount: amount_u256,
        };
        let approve_tx = TransactionRequest::default()
            .to(source_token_address)
            .input(approve_call.abi_encode().into());
        let approve_hash = send_and_confirm_transaction(&provider, approve_tx).await?;
        println!("Approval confirmed: {:#x}", approve_hash);
    }

    // Get adapter args and transaction value
    let (adapter_args, tx_value) = get_adapter_args_and_value(
        adapter,
        &provider,
        contract_address,
        adapter_address,
        PAYLOAD_TYPE_TOKEN_TRANSFER,
        &config,
    )
    .await?;

    // Encode the sendToken function call
    let call = Portal::sendTokenCall {
        amount: U256::from(amount),
        sourceToken: source_token_address,
        destinationChainId: config.solana_chain_id,
        destinationToken: FixedBytes::from(destination_token_bytes),
        recipient: FixedBytes::from(recipient_bytes),
        refundAddress: FixedBytes::from(refund_address),
        bridgeAdapter: adapter_address,
        bridgeAdapterArgs: adapter_args,
    };

    let tx = TransactionRequest::default()
        .to(contract_address)
        .input(call.abi_encode().into())
        .value(tx_value);

    // Send transaction
    let tx_hash = send_and_confirm_transaction(&provider, tx).await?;
    println!("Transaction hash: {:#x}", tx_hash);

    // Get final receipt status
    let receipt = provider
        .get_transaction_receipt(tx_hash)
        .await?
        .context("Transaction receipt not found")?;
    println!("Transaction status: {:?}", receipt.status());

    Ok(())
}
