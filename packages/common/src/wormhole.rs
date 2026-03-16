use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_client::rpc_client::RpcClient as BlockingRpcClient;
use anchor_lang::solana_program::{
    hash::hashv,
    instruction::{AccountMeta, Instruction},
};
use anchor_lang::{prelude::*, system_program};
use serde::{Deserialize, Serialize};

use crate::{
    consts::{WORMHOLE_BRIDGE_PROGRAM_ID, WORMHOLE_BRIDGE_PROGRAM_ID_DEVNET},
    pda,
    wormhole_adapter::{self, constants::EMITTER_SEED},
};

/// Wormhole executor related constants
pub const SOLANA_WORMHOLE_CHAIN_ID: u16 = 1;
pub const DEFAULT_GAS_LIMIT: u128 = 450_000;
pub const DEFAULT_MSG_VALUE: u128 = 4_500_000;
const EXECUTOR_PROGRAM_ID: Pubkey = pubkey!("execXUrAsMnqMmTHj5m7N1YQgsDz3cwGLYCYyuDRciV");
const EXECUTOR_QUOTE_API_URL_DEVNET: &str = "https://executor-testnet.labsapis.com/v0/quote";
const EXECUTOR_QUOTE_API_URL: &str = "https://executor.labsapis.com/v0/quote";
const GAS_INSTRUCTION_DISCRIMINANT: u8 = 1;
const REQ_VAA_V1: &[u8; 4] = b"ERV1";

/// Build a complete relay instruction for cross-chain execution.
///
/// This function handles the full relay setup:
/// 1. Creates relay instructions with gas parameters
/// 2. Fetches the executor quote from the relay API
/// 3. Decodes the signed quote and extracts the payee
/// 4. Builds the VAA request
/// 5. Constructs the final instruction
///
/// # Arguments
///
/// * `payer` - The account paying for the relay
/// * `destination_chain` - Wormhole chain ID of the destination
/// * `peer_portal` - 32-byte address of the portal on the destination chain
/// * `gas_limit` - Optional gas limit (defaults to [`DEFAULT_GAS_LIMIT`])
/// * `msg_value` - Optional message value (defaults to [`DEFAULT_MSG_VALUE`])
///
/// # Returns
///
/// Returns the instruction to request execution, or an error if the quote fetch fails.
pub async fn build_relay_instruction(
    payer: &Pubkey,
    destination_chain: u16,
    sequence: u64,
    peer_portal: &[u8; 32],
    gas_limit: Option<u128>,
    msg_value: Option<u128>,
    devnet: bool,
) -> std::result::Result<Instruction, ExecutorQuoteError> {
    let gas_limit = gas_limit.unwrap_or(DEFAULT_GAS_LIMIT);
    let msg_value = msg_value.unwrap_or(DEFAULT_MSG_VALUE);

    let relay_instructions = RelayInstructions::new(gas_limit, msg_value);
    let quote = fetch_executor_quote(
        SOLANA_WORMHOLE_CHAIN_ID,
        destination_chain,
        &relay_instructions,
        devnet,
    )
    .await?;

    let estimated_cost: u64 = quote
        .estimated_cost
        .as_ref()
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);

    let (signed_quote_bytes, payee) = decode_signed_quote(&quote.signed_quote)?;

    // VAA request
    let vaa_request = VaaRequest {
        emitter_chain: SOLANA_WORMHOLE_CHAIN_ID,
        emitter_address: pda!(&[EMITTER_SEED], &wormhole_adapter::ID).to_bytes(),
        sequence,
    };

    Ok(build_request_for_execution_ix(
        payer,
        &payee,
        estimated_cost,
        destination_chain,
        peer_portal,
        payer,
        &signed_quote_bytes,
        &vaa_request,
        &relay_instructions,
    ))
}

/// Convert an M0 chain ID to a Wormhole chain ID.
pub fn get_wormhole_chain_id(m0_chain_id: u32) -> Option<u16> {
    match m0_chain_id {
        // Mainnets
        1 => Some(2),          // Ethereum
        1399811149 => Some(1), // Solana Mainnet
        1399811150 => Some(1), // Solana Devnet
        42161 => Some(23),     // Arbitrum
        8453 => Some(30),      // Base
        999 => Some(47),       // Hyper EVM
        10 => Some(24),        // Optimism
        4326 => Some(64),      // MegaETH
        // Testnets
        421614 => Some(10003),   // Arbitrum Testnet
        11155111 => Some(10002), // Sepolia
        84532 => Some(10004),    // Base Testnet
        _ => None,
    }
}

fn get_sequence_account(devnet: bool) -> Pubkey {
    let emitter = pda!(&[EMITTER_SEED], &wormhole_adapter::ID);
    let wormhole_pid = if devnet {
        WORMHOLE_BRIDGE_PROGRAM_ID_DEVNET
    } else {
        WORMHOLE_BRIDGE_PROGRAM_ID
    };
    pda!(&[b"Sequence", &emitter.to_bytes()], &wormhole_pid)
}

fn parse_sequence(data: &[u8]) -> std::result::Result<u64, ExecutorQuoteError> {
    Ok(u64::from_le_bytes(data[..8].try_into().map_err(|_| {
        ExecutorQuoteError::ParseFailed("Invalid sequence data".to_string())
    })?))
}

pub async fn get_current_sequence(
    rpc_client: &RpcClient,
    devnet: bool,
) -> std::result::Result<u64, ExecutorQuoteError> {
    let sequence_data = rpc_client
        .get_account_data(&get_sequence_account(devnet))
        .await
        .map_err(|e| {
            ExecutorQuoteError::RequestFailed(format!("Failed to get sequence account: {}", e))
        })?;
    parse_sequence(&sequence_data)
}

pub fn get_current_sequence_blocking(
    rpc_client: &BlockingRpcClient,
    devnet: bool,
) -> std::result::Result<u64, ExecutorQuoteError> {
    let sequence_data = rpc_client
        .get_account_data(&get_sequence_account(devnet))
        .map_err(|e| {
            ExecutorQuoteError::RequestFailed(format!("Failed to get sequence account: {}", e))
        })?;
    parse_sequence(&sequence_data)
}

/// Error type for executor quote operations.
#[derive(Debug)]
pub enum ExecutorQuoteError {
    RequestFailed(String),
    ParseFailed(String),
    InvalidHex(String),
    InvalidQuoteFormat(String),
}

impl std::fmt::Display for ExecutorQuoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestFailed(e) => write!(f, "Failed to fetch executor quote: {}", e),
            Self::ParseFailed(e) => write!(f, "Failed to parse quote response: {}", e),
            Self::InvalidHex(e) => write!(f, "Invalid hex in signed quote: {}", e),
            Self::InvalidQuoteFormat(e) => write!(f, "Invalid quote format: {}", e),
        }
    }
}

impl std::error::Error for ExecutorQuoteError {}

/// Gas parameters for relay execution.
#[derive(Debug, Clone)]
struct RelayInstructions {
    gas_limit: u128,
    msg_value: u128,
}

impl RelayInstructions {
    fn new(gas_limit: u128, msg_value: u128) -> Self {
        Self {
            gas_limit,
            msg_value,
        }
    }

    /// Encode to bytes matching the executor's relayInstructionsLayout.
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(1 + 16 + 16);
        data.push(GAS_INSTRUCTION_DISCRIMINANT);
        data.extend_from_slice(&self.gas_limit.to_be_bytes());
        data.extend_from_slice(&self.msg_value.to_be_bytes());
        data
    }

    /// Encode to hex string with 0x prefix.
    fn encode_hex(&self) -> String {
        format!("0x{}", hex::encode(self.encode()))
    }
}

/// VAA request info for the executor.
#[derive(Debug, Clone)]
struct VaaRequest {
    emitter_chain: u16,
    emitter_address: [u8; 32],
    sequence: u64,
}

impl VaaRequest {
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(4 + 2 + 32 + 8);
        data.extend_from_slice(REQ_VAA_V1);
        data.extend_from_slice(&self.emitter_chain.to_be_bytes());
        data.extend_from_slice(&self.emitter_address);
        data.extend_from_slice(&self.sequence.to_be_bytes());
        data
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuoteRequest {
    src_chain: u16,
    dst_chain: u16,
    relay_instructions: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuoteResponse {
    signed_quote: String,
    estimated_cost: Option<String>,
}

async fn fetch_executor_quote(
    src_chain: u16,
    dst_chain: u16,
    relay_instructions: &RelayInstructions,
    devnet: bool,
) -> std::result::Result<QuoteResponse, ExecutorQuoteError> {
    let request = QuoteRequest {
        src_chain,
        dst_chain,
        relay_instructions: relay_instructions.encode_hex(),
    };

    let response_text = reqwest::Client::new()
        .post(if devnet {
            EXECUTOR_QUOTE_API_URL_DEVNET
        } else {
            EXECUTOR_QUOTE_API_URL
        })
        .json(&request)
        .send()
        .await
        .map_err(|e| ExecutorQuoteError::RequestFailed(e.to_string()))?
        .text()
        .await
        .map_err(|e| ExecutorQuoteError::RequestFailed(e.to_string()))?;

    serde_json::from_str(&response_text)
        .map_err(|e| ExecutorQuoteError::ParseFailed(format!("{}: {}", e, response_text)))
}

/// Signed quote layout offsets
const QUOTE_ID_SIZE: usize = 4;
const QUOTER_ADDRESS_SIZE: usize = 20;
const PAYEE_ADDRESS_OFFSET: usize = QUOTE_ID_SIZE + QUOTER_ADDRESS_SIZE;
const PAYEE_ADDRESS_SIZE: usize = 32;

fn decode_signed_quote(
    signed_quote_hex: &str,
) -> std::result::Result<(Vec<u8>, Pubkey), ExecutorQuoteError> {
    let hex_str = signed_quote_hex
        .strip_prefix("0x")
        .unwrap_or(signed_quote_hex);

    let quote_bytes =
        hex::decode(hex_str).map_err(|e| ExecutorQuoteError::InvalidHex(e.to_string()))?;

    let expected_len = PAYEE_ADDRESS_OFFSET + PAYEE_ADDRESS_SIZE;
    if quote_bytes.len() < expected_len {
        return Err(ExecutorQuoteError::InvalidQuoteFormat(format!(
            "quote too short: expected at least {} bytes, got {}",
            expected_len,
            quote_bytes.len()
        )));
    }

    let payee_bytes: [u8; 32] = quote_bytes
        [PAYEE_ADDRESS_OFFSET..PAYEE_ADDRESS_OFFSET + PAYEE_ADDRESS_SIZE]
        .try_into()
        .map_err(|_| {
            ExecutorQuoteError::InvalidQuoteFormat("payee address slice is not 32 bytes".into())
        })?;

    Ok((quote_bytes, Pubkey::new_from_array(payee_bytes)))
}

fn request_for_execution_discriminator() -> [u8; 8] {
    let hash = hashv(&[b"global:request_for_execution"]);
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash.as_ref()[0..8]);
    discriminator
}

fn build_request_for_execution_ix(
    payer: &Pubkey,
    payee: &Pubkey,
    estimated_cost: u64,
    destination_chain: u16,
    peer_portal: &[u8; 32],
    refund_address: &Pubkey,
    signed_quote: &[u8],
    vaa_request: &VaaRequest,
    relay_instructions: &RelayInstructions,
) -> Instruction {
    let mut data = Vec::new();

    data.extend_from_slice(&request_for_execution_discriminator());
    data.extend_from_slice(&estimated_cost.to_le_bytes());
    data.extend_from_slice(&destination_chain.to_le_bytes());
    data.extend_from_slice(peer_portal);
    data.extend_from_slice(refund_address.as_ref());

    // Signed quote with length prefix
    data.extend_from_slice(&(signed_quote.len() as u32).to_le_bytes());
    data.extend_from_slice(signed_quote);

    // VAA request with length prefix
    let vaa_request_bytes = vaa_request.encode();
    data.extend_from_slice(&(vaa_request_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(&vaa_request_bytes);

    // Relay instructions with length prefix
    let relay_instructions_bytes = relay_instructions.encode();
    data.extend_from_slice(&(relay_instructions_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(&relay_instructions_bytes);

    Instruction {
        program_id: EXECUTOR_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(*payee, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WormholeResponse {
    pub operations: Vec<Operation>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub emitter_chain: i64,
    pub sequence: String,
    pub vaa: Vaa,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vaa {
    pub raw: String,
    pub guardian_set_index: i64,
    pub is_duplicated: bool,
}

pub type ExecutorTransactions = Vec<ExecutorTransaction>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutorTransaction {
    pub chain_id: i64,
    pub id: String,
    pub failure_cause: String,
    pub failure_message: String,
    pub status: String,
    pub tx_hash: String,
    pub txs: Vec<Tx>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tx {
    pub tx_hash: String,
    pub chain_id: i64,
    pub block_number: String,
    pub block_time: String,
    pub cost: String,
}
