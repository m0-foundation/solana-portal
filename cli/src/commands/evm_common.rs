use alloy::{
    network::EthereumWallet,
    primitives::{Address, Bytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    sol_types::{SolCall, SolValue},
    transports::http,
};
use anyhow::{Context, Result};
use m0_portal_common::{DEFAULT_GAS_LIMIT, DEFAULT_MSG_VALUE, SOLANA_WORMHOLE_CHAIN_ID};
use serde::{Deserialize, Serialize};

use crate::{
    types::evm::{Portal, EVM_HYPERLANE_ADAPTER, EVM_PORTAL_CONTRACT, EVM_WORMHOLE_ADAPTER},
    BridgeAdapter, Network,
};

const GAS_INSTRUCTION_DISCRIMINANT: u8 = 1;

/// All network-dependent EVM configuration values
pub struct NetworkConfig {
    pub rpc_url: String,
    pub wormhole_source_chain_id: u16,
    pub executor_quote_api_url: &'static str,
    pub solana_chain_id: u32,
    pub network_label: &'static str,
}

impl NetworkConfig {
    pub fn from_network(network: Network) -> anyhow::Result<Self> {
        match network {
            Network::Devnet | Network::Testnet => Ok(Self {
                rpc_url: std::env::var("EVM_RPC_URL")
                    .unwrap_or_else(|_| "https://sepolia.gateway.tenderly.co".to_string()),
                wormhole_source_chain_id: 10002,
                executor_quote_api_url: "https://executor-testnet.labsapis.com/v0/quote",
                solana_chain_id: 1399811150,
                network_label: "devnet (Sepolia -> Solana devnet)",
            }),
            Network::Mainnet => {
                let rpc_url = std::env::var("EVM_RPC_URL")
                    .context("EVM_RPC_URL environment variable is required for mainnet")?;
                Ok(Self {
                    rpc_url,
                    wormhole_source_chain_id: 2,
                    executor_quote_api_url: "https://executor.labsapis.com/v0/quote",
                    solana_chain_id: 1399811149,
                    network_label: "mainnet (Ethereum -> Solana mainnet)",
                })
            }
        }
    }
}

/// Request body for the executor quote API
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuoteRequest {
    src_chain: u16,
    dst_chain: u16,
    relay_instructions: String,
}

/// Response from the executor quote API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuoteResponse {
    signed_quote: String,
    estimated_cost: Option<String>,
}

/// Result from fetching a Wormhole executor quote
pub struct WormholeQuote {
    pub signed_quote: Vec<u8>,
    pub estimated_cost: U256,
}

/// Load private key from EVM_KEY environment variable
pub fn load_private_key() -> PrivateKeySigner {
    let private_key = std::env::var("EVM_KEY").expect("EVM_KEY environment variable not set");

    let private_key = private_key.trim_start_matches("0x");

    let key = private_key.parse::<PrivateKeySigner>().unwrap();
    println!("Loaded private key with address: {:#x}", key.address());

    key
}

/// Create a provider with the given signer
pub fn create_provider(
    signer: PrivateKeySigner,
    config: &NetworkConfig,
) -> Result<impl Provider<http::Http<http::Client>>> {
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(config.rpc_url.parse()?);
    Ok(provider)
}

/// Get the portal contract address
pub fn get_portal_address() -> Result<Address> {
    EVM_PORTAL_CONTRACT
        .parse::<Address>()
        .context("Failed to parse portal contract address")
}

/// Get adapter address based on bridge selection
pub fn get_adapter_address(adapter: BridgeAdapter) -> Result<Address> {
    let addr_str = match adapter {
        BridgeAdapter::Hyperlane => EVM_HYPERLANE_ADAPTER,
        BridgeAdapter::Wormhole => EVM_WORMHOLE_ADAPTER,
    };
    addr_str
        .parse::<Address>()
        .context("Failed to parse adapter address")
}

/// Get adapter name for display
pub fn get_adapter_name(adapter: BridgeAdapter) -> String {
    match adapter {
        BridgeAdapter::Hyperlane => "Hyperlane".to_string(),
        BridgeAdapter::Wormhole => "Wormhole".to_string(),
    }
}

/// Estimate gas fee by calling the quote function
pub async fn estimate_gas_fee<P>(
    provider: &P,
    contract_address: Address,
    adapter_address: Address,
    payload_type: u8,
    config: &NetworkConfig,
) -> Result<U256>
where
    P: Provider<http::Http<http::Client>>,
{
    let call = Portal::quoteCall {
        destinationChainId: config.solana_chain_id,
        payloadType: payload_type,
        bridgeAdapter: adapter_address,
    };

    let calldata = call.abi_encode();

    let tx = TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    let result = provider
        .call(&tx)
        .await
        .context("Failed to call quote function")?;

    let fee = U256::abi_decode(&result, true).context("Failed to decode quote result")?;

    Ok(fee)
}

/// Format wei to ETH string
pub fn format_wei_to_eth(wei: U256) -> String {
    let eth = wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
    format!("{:.6}", eth)
}

/// Encode relay instructions for the executor quote API
fn encode_relay_instructions(gas_limit: u128, msg_value: u128) -> String {
    let mut data = Vec::with_capacity(33);
    data.push(GAS_INSTRUCTION_DISCRIMINANT);
    data.extend_from_slice(&gas_limit.to_be_bytes());
    data.extend_from_slice(&msg_value.to_be_bytes());
    format!("0x{}", hex::encode(data))
}

/// Fetch a signed quote from the Wormhole executor API
pub async fn fetch_wormhole_quote(config: &NetworkConfig) -> Result<WormholeQuote> {
    println!("Fetching Wormhole executor quote...");

    let relay_instructions = encode_relay_instructions(DEFAULT_GAS_LIMIT, DEFAULT_MSG_VALUE);

    let request = QuoteRequest {
        src_chain: config.wormhole_source_chain_id,
        dst_chain: SOLANA_WORMHOLE_CHAIN_ID,
        relay_instructions,
    };

    let client = reqwest::Client::new();
    let response: QuoteResponse = client
        .post(config.executor_quote_api_url)
        .json(&request)
        .send()
        .await
        .context("Failed to fetch executor quote")?
        .json()
        .await
        .context("Failed to parse executor quote response")?;

    // Decode hex to bytes (strip 0x prefix if present)
    let hex_str = response
        .signed_quote
        .strip_prefix("0x")
        .unwrap_or(&response.signed_quote);
    let signed_quote = hex::decode(hex_str).context("Failed to decode signed quote hex")?;

    // Parse estimated cost (defaults to 0 if not provided)
    let estimated_cost = response
        .estimated_cost
        .as_ref()
        .and_then(|c| c.parse::<u128>().ok())
        .map(U256::from)
        .unwrap_or(U256::ZERO);

    println!(
        "Got signed quote ({} bytes), estimated cost: {} wei",
        signed_quote.len(),
        estimated_cost
    );

    Ok(WormholeQuote {
        signed_quote,
        estimated_cost,
    })
}

/// Get adapter args and transaction value based on adapter type
pub async fn get_adapter_args_and_value<P>(
    adapter: BridgeAdapter,
    provider: &P,
    contract_address: Address,
    adapter_address: Address,
    payload_type: u8,
    config: &NetworkConfig,
) -> Result<(Bytes, U256)>
where
    P: Provider<http::Http<http::Client>>,
{
    match adapter {
        BridgeAdapter::Wormhole => {
            let quote = fetch_wormhole_quote(config).await?;
            let value_eth = format_wei_to_eth(quote.estimated_cost);
            println!("Estimated cost: {} ETH", value_eth);
            Ok((Bytes::from(quote.signed_quote), quote.estimated_cost))
        }
        BridgeAdapter::Hyperlane => {
            let gas_fee = estimate_gas_fee(
                provider,
                contract_address,
                adapter_address,
                payload_type,
                config,
            )
            .await?;
            let gas_fee_eth = format_wei_to_eth(gas_fee);
            println!("Estimated gas fee: {} ETH", gas_fee_eth);
            Ok((Bytes::new(), gas_fee))
        }
    }
}

/// Send a transaction and wait for receipt
pub async fn send_and_confirm_transaction<P>(
    provider: &P,
    tx: TransactionRequest,
) -> Result<alloy::primitives::TxHash>
where
    P: Provider<http::Http<http::Client>>,
{
    // Fetch base fee from latest block and priority fee for EIP-1559 fee calculation
    let latest_block = provider
        .get_block_by_number(alloy::eips::BlockNumberOrTag::Latest, false.into())
        .await
        .context("Failed to get latest block")?
        .context("Latest block not found")?;
    let base_fee = latest_block
        .header
        .base_fee_per_gas
        .context("Latest block missing base_fee_per_gas (non-EIP-1559 network?)")?;
    let max_priority_fee = provider
        .get_max_priority_fee_per_gas()
        .await
        .context("Failed to get max priority fee")?;
    let bumped_priority_fee = max_priority_fee * 120 / 100;

    // 2x base fee buffer ensures max_fee stays above base fee even if it spikes next block
    let max_fee = base_fee as u128 * 2 + bumped_priority_fee;

    println!(
        "Gas prices: maxFee={} gwei, priorityFee={} gwei (bumped 20%)",
        max_fee / 1_000_000_000,
        bumped_priority_fee / 1_000_000_000,
    );

    let tx = tx
        .max_fee_per_gas(max_fee)
        .max_priority_fee_per_gas(bumped_priority_fee);

    println!("Sending transaction...");
    let pending_tx = provider
        .send_transaction(tx)
        .await
        .context("Failed to send transaction")?;

    let tx_hash = *pending_tx.tx_hash();
    println!("Transaction sent: {:#x}", tx_hash);
    println!("Waiting for confirmation...");

    let receipt = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        pending_tx.get_receipt(),
    )
    .await
    .context("Transaction confirmation timed out after 5 minutes")?
    .context("Failed to get transaction receipt")?;

    Ok(receipt.transaction_hash)
}
