use alloy::{
    network::EthereumWallet,
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    sol_types::{SolCall, SolValue},
    transports::http,
};
use anyhow::{Context, Result};

use crate::{
    types::evm::{
        address_to_bytes32, Portal, MTOKEN_INDEX_PAYLOAD_TYPE, SEPOLIA_HYPERLANE_ADAPTER,
        SEPOLIA_PORTAL_CONTRACT, SEPOLIA_WORMHOLE_ADAPTER, SOLANA_CHAIN_ID,
    },
    BridgeAdapter,
};

const DEFAULT_SEPOLIA_RPC: &str =
    "https://eth-sepolia.g.alchemy.com/v2/w-r8VabcoQMvw_Sp-krrirztyoLSc2sS";

/// Send $M index from Sepolia to Solana via the Portal contract
pub async fn send_evm_index(adapter: BridgeAdapter) -> Result<()> {
    let adapter_name = match adapter {
        BridgeAdapter::Hyperlane => "Hyperlane (Sepolia)",
        BridgeAdapter::Wormhole => "Wormhole (Sepolia)",
    };

    println!("Using adapter: {}", adapter_name);

    // Load private key from environment
    let signer = load_private_key()
        .context("Failed to load private key. Make sure PRIVATE_KEY env var is set")?;

    let sender_address = signer.address();
    println!("Sender address: {}", sender_address);

    // Create provider with signer
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(DEFAULT_SEPOLIA_RPC.parse()?);

    let contract_address = SEPOLIA_PORTAL_CONTRACT.parse::<Address>()?;
    let adapter_address = get_adapter_address(adapter)?;
    let refund_address = address_to_bytes32(sender_address);

    println!("Bridge adapter: {}", adapter_address);

    let gas_fee = estimate_gas_fee(&provider, contract_address, adapter_address).await?;
    let gas_fee_eth = format_wei_to_eth(gas_fee);
    println!("Estimated gas fee: {} ETH", gas_fee_eth);

    println!(
        "Sending mToken index to Solana (chain ID: {})...",
        SOLANA_CHAIN_ID
    );

    // Send transaction
    let tx_hash = send_transaction(
        &provider,
        contract_address,
        refund_address,
        adapter_address,
        gas_fee,
    )
    .await?;

    println!("Transaction hash: {:#x}", tx_hash);
    println!("Waiting for confirmation...");

    // Wait for transaction receipt
    let receipt = provider
        .get_transaction_receipt(tx_hash)
        .await?
        .context("Transaction receipt not found")?;

    println!("Transaction status: {:?}", receipt.status());

    Ok(())
}

/// Load private key from PRIVATE_KEY environment variable
fn load_private_key() -> Result<PrivateKeySigner> {
    let private_key =
        std::env::var("PRIVATE_KEY").context("PRIVATE_KEY environment variable not set")?;

    let private_key = private_key.trim_start_matches("0x");

    private_key
        .parse::<PrivateKeySigner>()
        .context("Invalid private key format")
}

/// Get adapter address based on bridge selection
fn get_adapter_address(adapter: BridgeAdapter) -> Result<Address> {
    let addr_str = match adapter {
        BridgeAdapter::Hyperlane => SEPOLIA_HYPERLANE_ADAPTER,
        BridgeAdapter::Wormhole => SEPOLIA_WORMHOLE_ADAPTER,
    };
    addr_str
        .parse::<Address>()
        .context("Failed to parse adapter address")
}

/// Estimate gas fee by calling the quote function
async fn estimate_gas_fee<P>(
    provider: &P,
    contract_address: Address,
    adapter_address: Address,
) -> Result<U256>
where
    P: Provider<http::Http<http::Client>>,
{
    // Encode the quote function call
    let call = Portal::quoteCall {
        destinationChainId: SOLANA_CHAIN_ID,
        payloadType: MTOKEN_INDEX_PAYLOAD_TYPE,
        bridgeAdapter: adapter_address,
    };

    let calldata = call.abi_encode();

    // Create a call request
    let tx = TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    // Call the contract
    let result = provider
        .call(&tx)
        .await
        .context("Failed to call quote function")?;

    // Decode the result (uint256)
    let fee = U256::abi_decode(&result, true).context("Failed to decode quote result")?;

    Ok(fee)
}

/// Send the sendMTokenIndex transaction
async fn send_transaction<P>(
    provider: &P,
    contract_address: Address,
    refund_address: [u8; 32],
    adapter_address: Address,
    value: U256,
) -> Result<alloy::primitives::TxHash>
where
    P: Provider<http::Http<http::Client>>,
{
    // Empty bytes for bridgeAdapterArgs
    let adapter_args = Bytes::new();

    // Encode the sendMTokenIndex function call
    let call = Portal::sendMTokenIndexCall {
        destinationChainId: SOLANA_CHAIN_ID,
        refundAddress: FixedBytes::from(refund_address),
        bridgeAdapter: adapter_address,
        bridgeAdapterArgs: adapter_args,
    };

    let calldata = call.abi_encode();

    // Create transaction request
    let tx = TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into())
        .value(value);

    // Send the transaction
    let pending_tx = provider
        .send_transaction(tx)
        .await
        .context("Failed to send transaction")?;

    // Wait for confirmation
    let receipt = pending_tx
        .get_receipt()
        .await
        .context("Failed to get transaction receipt")?;

    Ok(receipt.transaction_hash)
}

/// Format wei to ETH string
fn format_wei_to_eth(wei: U256) -> String {
    let eth = wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
    format!("{:.6}", eth)
}
