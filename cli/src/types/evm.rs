use alloy::primitives::Address;

// Sepolia Portal and adapters
pub const SEPOLIA_PORTAL_CONTRACT: &str = "0x50D65829Eae411B655bAA92539E4F8c46D20638C";
pub const SEPOLIA_HYPERLANE_ADAPTER: &str = "0xFc44dadD758A7737aC9200059e9FCd1521d75a07";
pub const SEPOLIA_WORMHOLE_ADAPTER: &str = "0x6b2A7bFa5F1C03EbFae779Df6988b8aC14CA4155";

// Destination chain ID for Solana
pub const SOLANA_CHAIN_ID: u32 = 1399811150;

// PayloadType enum value for MTokenIndex
pub const MTOKEN_INDEX_PAYLOAD_TYPE: u8 = 1;

// Contract ABI bindings using alloy's sol! macro
alloy::sol! {
    interface IPortal {
        // sendMTokenIndex with explicit bridge adapter
        function sendMTokenIndex(
            uint32 destinationChainId,
            bytes32 refundAddress,
            address bridgeAdapter,
            bytes bridgeAdapterArgs
        ) external payable returns (bytes32 messageId);

        // quote function to estimate gas fees
        function quote(
            uint32 destinationChainId,
            uint8 payloadType,
            address bridgeAdapter
        ) external view returns (uint256);

        // Event emitted when mToken index is sent
        event MTokenIndexSent(
            uint32 indexed destinationChainId,
            uint128 index,
            address bridgeAdapter,
            bytes32 messageId
        );
    }
}

pub use IPortal as Portal;

/// Convert an Ethereum address to bytes32 format (left-padded with zeros)
/// This is needed for Solana addresses in cross-chain messages
pub fn address_to_bytes32(addr: Address) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[12..].copy_from_slice(addr.as_slice());
    bytes
}
