use anchor_lang::prelude::*;
use anchor_spl::associated_token::spl_associated_token_account::solana_program::keccak;

pub const ETHEREUM_CHAIN_ID: u32 = 1;
pub const SEPOLIA_CHAIN_ID: u32 = 11155111;

#[constant]
pub const GLOBAL_SEED: &[u8] = b"global";
#[constant]
pub const M_VAULT_SEED: &[u8] = b"m_vault";
#[constant]
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";
#[constant]
pub use m0_portal_common::interfaces::AUTHORITY_SEED;
#[constant]
pub const MESSAGE_SEED: &[u8] = b"message";
#[constant]
pub const CHAIN_PATHS_SEED: &[u8] = b"chain_paths";

#[account]
#[derive(InitSpace)]
pub struct PortalGlobal {
    pub bump: u8,
    pub chain_id: u32,
    pub m_mint: Pubkey,
    pub admin: Pubkey,
    pub outgoing_paused: bool,
    pub incoming_paused: bool,
    pub m_index: u128,
    pub message_nonce: u64,
    pub pending_admin: Option<Pubkey>,
    pub isolated_hub_chain_id: Option<u32>,
    pub padding: [u8; 128],
}

impl PortalGlobal {
    pub fn generate_message_id(&mut self, destination_chain_id: u32) -> [u8; 32] {
        self.message_nonce += 1;

        let mut encoded = [0u8; 96];

        // ABI encode: each value is padded to 32 bytes (left-padded for integers)
        encoded[28..32].copy_from_slice(&self.chain_id.to_be_bytes());
        encoded[60..64].copy_from_slice(&destination_chain_id.to_be_bytes());
        encoded[88..96].copy_from_slice(&self.message_nonce.to_be_bytes());

        keccak::hash(&encoded).to_bytes()
    }
}

impl PortalGlobal {
    pub const SIZE: usize = PortalGlobal::INIT_SPACE + PortalGlobal::DISCRIMINATOR.len();
}

#[account]
#[derive(InitSpace)]
pub struct BridgeMessage {
    pub consumed: bool,
}

impl BridgeMessage {
    pub const SIZE: usize = BridgeMessage::INIT_SPACE + BridgeMessage::DISCRIMINATOR.len();
}

/// Represents an allowed bridging path from a source token to a destination token
#[derive(Clone, AnchorSerialize, AnchorDeserialize, InitSpace, PartialEq, Debug)]
pub struct BridgePath {
    /// Extension mint on Solana (e.g., wM mint pubkey)
    pub source_mint: Pubkey,
    /// Token address on destination chain (e.g., Ethereum wM address)
    pub destination_token: [u8; 32],
}

impl BridgePath {
    pub const SIZE: usize = 32 + 32; // 64 bytes
}

/// Per-destination-chain configuration of allowed bridging paths
#[account]
#[derive(InitSpace)]
pub struct ChainBridgePaths {
    pub bump: u8,
    pub destination_chain_id: u32,
    #[max_len(20)]
    pub paths: Vec<BridgePath>,
}

impl ChainBridgePaths {
    /// Check if a given source_mint → destination_token path is supported
    pub fn is_path_supported(&self, source_mint: &Pubkey, destination_token: &[u8; 32]) -> bool {
        self.paths
            .iter()
            .any(|p| p.source_mint == *source_mint && p.destination_token == *destination_token)
    }

    /// Calculate account size for a given number of paths
    pub fn size(num_paths: usize) -> usize {
        8 +  // discriminator
        1 +  // bump
        4 +  // destination_chain_id
        4 +  // Vec length prefix
        (num_paths * BridgePath::SIZE)
    }
}

#[event]
pub struct MTokenIndexReceived {
    pub index: u128,
    pub message_id: [u8; 32],
}
#[cfg(test)]
mod tests {
    use super::*;
    use alloy_sol_types::{sol, SolType};

    #[test]
    fn test_generate_message_id_matches_solidity() {
        let chain_id: u32 = 1;
        let destination_chain_id: u32 = 2;
        let nonce_value: u64 = 1;

        // Create a mock PortalGlobal instance
        let mut portal_global = PortalGlobal {
            bump: 0,
            chain_id,
            admin: Pubkey::default(),
            m_mint: Pubkey::default(),
            outgoing_paused: false,
            incoming_paused: false,
            m_index: 0,
            message_nonce: nonce_value - 1, // Will be incremented
            pending_admin: None,
            isolated_hub_chain_id: None,
            padding: [0u8; 128],
        };

        // Generate message ID using Rust function
        let rust_message_id = portal_global.generate_message_id(destination_chain_id);

        // Generate expected hash using Solidity-like encoding
        type SolidityTypes = (sol! { uint32 }, sol! { uint32 }, sol! { uint256 });
        let nonce_u256 = alloy_sol_types::private::U256::from(nonce_value);
        let encoded = SolidityTypes::abi_encode(&(chain_id, destination_chain_id, nonce_u256));
        let expected_hash = alloy_sol_types::private::keccak256(&encoded);

        assert_eq!(
            rust_message_id, expected_hash.0,
            "Rust generate_message_id output doesn't match Solidity keccak256(abi.encode(...))"
        );
    }

    #[test]
    fn test_generate_message_id_multiple() {
        let test_cases = vec![
            (1u32, 2u32, 1u64),
            (1u32, 137u32, 5u64),
            (42u32, 100u32, 1000u64),
            (u32::MAX, u32::MAX, u64::MAX),
        ];

        for (chain_id, dest_chain_id, nonce) in test_cases {
            let mut portal_global = PortalGlobal {
                bump: 0,
                chain_id,
                admin: Pubkey::default(),
                m_mint: Pubkey::default(),
                outgoing_paused: false,
                incoming_paused: false,
                m_index: 0,
                message_nonce: nonce - 1,
                pending_admin: None,
                isolated_hub_chain_id: None,
                padding: [0u8; 128],
            };

            let rust_message_id = portal_global.generate_message_id(dest_chain_id);

            // Generate expected hash
            type SolidityTypes = (sol! { uint32 }, sol! { uint32 }, sol! { uint256 });
            let nonce_u256 = alloy_sol_types::private::U256::from(nonce);
            let encoded = SolidityTypes::abi_encode(&(chain_id, dest_chain_id, nonce_u256));
            let expected_hash = alloy_sol_types::private::keccak256(&encoded);

            assert_eq!(
                rust_message_id, expected_hash.0,
                "Mismatch for chain_id={}, dest_chain_id={}, nonce={}",
                chain_id, dest_chain_id, nonce
            );
        }
    }
}
