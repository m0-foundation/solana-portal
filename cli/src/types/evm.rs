use alloy::primitives::Address;

// Sepolia Portal and adapters
pub const EVM_PORTAL_CONTRACT: &str = "0xD925C84b55E4e44a53749fF5F2a5A13F63D128fd";
pub const EVM_HYPERLANE_ADAPTER: &str = "0xfCc1d596Ad6cAb0b5394eAa447d8626813180f32";
pub const EVM_WORMHOLE_ADAPTER: &str = "0xaCffEC28C4eEe21C889a4e6C0704c540Ed9D4fDd";

pub const PAYLOAD_TYPE_TOKEN_TRANSFER: u8 = 0;
pub const PAYLOAD_TYPE_INDEX: u8 = 1;

// Contract ABI bindings using alloy's sol! macro
alloy::sol! {
    /// IERC20 interface - standard ERC20 token functions
    interface IERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
    }

    /// IPortal interface - base portal contract
    interface IPortal {
        /// Transfers token to the destination chain using the specified bridge adapter
        function sendToken(
            uint256 amount,
            address sourceToken,
            uint32 destinationChainId,
            bytes32 destinationToken,
            bytes32 recipient,
            bytes32 refundAddress,
            address bridgeAdapter,
            bytes bridgeAdapterArgs
        ) external payable returns (bytes32 messageId);

        /// Returns the fee for delivering a cross-chain message using the specified bridge adapter
        function quote(
            uint32 destinationChainId,
            uint8 payloadType,
            address bridgeAdapter
        ) external view returns (uint256);

        /// Event emitted when token is sent to a destination chain
        event TokenSent(
            address indexed sourceToken,
            uint32 destinationChainId,
            bytes32 destinationToken,
            address indexed sender,
            bytes32 indexed recipient,
            uint256 amount,
            uint128 index,
            address bridgeAdapter,
            bytes32 messageId
        );
    }

    /// IHubPortal interface - extends IPortal with Hub-specific functions
    interface IHubPortal {
        /// Sends the $M token index to the destination chain using the specified bridge adapter
        function sendMTokenIndex(
            uint32 destinationChainId,
            bytes32 refundAddress,
            address bridgeAdapter,
            bytes bridgeAdapterArgs
        ) external payable returns (bytes32 messageId);

        /// Event emitted when the M token index is sent
        event MTokenIndexSent(
            uint32 indexed destinationChainId,
            uint128 index,
            address bridgeAdapter,
            bytes32 messageId
        );
    }
}

pub use IERC20 as Erc20;
pub use IHubPortal as HubPortal;
pub use IPortal as Portal;

/// Convert an Ethereum address to bytes32 format (left-padded with zeros)
/// This is needed for Solana addresses in cross-chain messages
pub fn address_to_bytes32(addr: Address) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[12..].copy_from_slice(addr.as_slice());
    bytes
}
