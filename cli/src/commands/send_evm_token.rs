use alloy::{
    primitives::{Address, FixedBytes, U256},
    providers::Provider,
    rpc::types::TransactionRequest,
    sol_types::SolCall,
};
use anyhow::{Context, Result};

use crate::{
    commands::common::parse_recipient,
    types::evm::{address_to_bytes32, Portal, PAYLOAD_TYPE_TOKEN_TRANSFER, SOLANA_CHAIN_ID},
    BridgeAdapter,
};

use super::evm_common::{
    create_provider, get_adapter_address, get_adapter_args_and_value, get_adapter_name,
    get_portal_address, load_private_key, send_and_confirm_transaction,
};

/// Send token transfer from Sepolia to Solana via the Portal contract
pub async fn send_evm_token(amount: u128, recipient: String, adapter: BridgeAdapter) -> Result<()> {
    println!("Using adapter: {}", get_adapter_name(adapter));
    println!("Sending {} tokens to Solana", amount);

    // Load private key and create provider
    let signer = load_private_key()
        .context("Failed to load private key. Make sure PRIVATE_KEY env var is set")?;
    let sender_address = signer.address();
    let provider = create_provider(signer)?;

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

    // Get adapter args and transaction value
    let (adapter_args, tx_value) = get_adapter_args_and_value(
        adapter,
        &provider,
        contract_address,
        adapter_address,
        PAYLOAD_TYPE_TOKEN_TRANSFER,
    )
    .await?;

    // Encode the sendToken function call
    let call = Portal::sendTokenCall {
        amount: U256::from(amount),
        sourceToken: source_token_address,
        destinationChainId: SOLANA_CHAIN_ID,
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
