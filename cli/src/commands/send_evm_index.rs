use alloy::{
    primitives::FixedBytes,
    providers::Provider,
    rpc::types::TransactionRequest,
    sol_types::SolCall,
};
use anyhow::{Context, Result};

use crate::{
    types::evm::{address_to_bytes32, HubPortal, PAYLOAD_TYPE_INDEX, SOLANA_CHAIN_ID},
    BridgeAdapter,
};

use super::evm_common::{
    create_provider, get_adapter_address, get_adapter_args_and_value, get_adapter_name,
    get_portal_address, load_private_key, send_and_confirm_transaction,
};

/// Send $M index from Sepolia to Solana via the HubPortal contract
pub async fn send_evm_index(adapter: BridgeAdapter) -> Result<()> {
    println!("Using adapter: {}", get_adapter_name(adapter));

    // Load private key and create provider
    let signer = load_private_key()
        .context("Failed to load private key. Make sure PRIVATE_KEY env var is set")?;
    let sender_address = signer.address();
    let provider = create_provider(signer)?;

    // Get addresses
    let contract_address = get_portal_address()?;
    let adapter_address = get_adapter_address(adapter)?;
    let refund_address = address_to_bytes32(sender_address);

    // Get adapter args and transaction value
    let (adapter_args, tx_value) = get_adapter_args_and_value(
        adapter,
        &provider,
        contract_address,
        adapter_address,
        PAYLOAD_TYPE_INDEX,
    )
    .await?;

    // Encode the sendMTokenIndex function call (HubPortal-specific)
    let call = HubPortal::sendMTokenIndexCall {
        destinationChainId: SOLANA_CHAIN_ID,
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
