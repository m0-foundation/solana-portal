# Isolated Bridge Paths for M Extension Tokens

## Overview

Add configurable bridge path isolation to ensure M extension tokens from one issuer's ecosystem only bridge to extensions within that same ecosystem, preventing cross-issuer token leakage. This mirrors the `supportedBridgingPath` pattern from the EVM M Portal Lite implementation.

**Problem**: Currently, the portal allows any whitelisted extension to bridge to any destination token. With multiple M extension issuers (e.g., wM, yM from different parties), we need to ensure tokens stay within their issuer's ecosystem.

**Solution**: Introduce per-destination-chain PDAs that store allowed source→destination token paths. Validation happens during `send_token` to ensure only configured paths are permitted.

## Architecture

### Data Model

```
┌─────────────────────────────────────────────────────────────┐
│                     PortalGlobal                            │
│  (existing - unchanged)                                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ has_one = admin
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              ChainBridgePaths (NEW)                         │
│  PDA: [CHAIN_PATHS_SEED, dest_chain_id]                    │
├─────────────────────────────────────────────────────────────┤
│  bump: u8                                                   │
│  destination_chain_id: u32                                  │
│  paths: Vec<BridgePath>                                      │
│    ├─ BridgePath { source_mint, destination_token }         │
│    ├─ BridgePath { source_mint, destination_token }         │
│    └─ ...                                                  │
└─────────────────────────────────────────────────────────────┘
```

### Message Flow

```
User calls send_token(amount, dest_token, dest_chain_id, recipient)
    │
    ▼
┌─────────────────────────────────────────┐
│ 1. Derive ChainBridgePaths PDA          │
│    seeds = [CHAIN_PATHS_SEED,           │
│             dest_chain_id.to_be_bytes()]│
└─────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────┐
│ 2. Check path exists in chain_paths     │
│    source_mint == extension_mint.key()  │
│    destination_token == dest_token      │
└─────────────────────────────────────────┘
    │
    ├─── Path NOT found → Error: UnsupportedBridgePath
    │
    ▼ Path found
┌─────────────────────────────────────────┐
│ 3. Continue with existing send_token    │
│    logic (unwrap, burn, send message)   │
└─────────────────────────────────────────┘
```

### Why Hybrid PDA-per-Chain Approach

| Approach            | Pros                                                | Cons                                     |
| ------------------- | --------------------------------------------------- | ---------------------------------------- |
| Vec in PortalGlobal | Simple, single account                              | O(n) scan all paths, realloc affects all |
| PDA per path        | O(1) lookup                                         | Many accounts (30+), higher base tx cost |
| **PDA per chain**   | O(1) chain lookup, O(5) path scan, isolated realloc | Slightly more complex than Vec           |

With 5-6 chains and 2-5 paths per chain:

- 5-6 PDAs total (manageable)
- ~2,000-2,500 CU per validation (efficient)
- Realloc only affects one chain

## New Account Types

### ChainBridgePaths

```rust
pub const CHAIN_PATHS_SEED: &[u8] = b"chain_paths";

#[account]
#[derive(InitSpace)]
pub struct ChainBridgePaths {
    /// PDA bump seed
    pub bump: u8,
    /// Target chain ID this config applies to
    pub destination_chain_id: u32,
    /// Allowed source→destination token paths
    #[max_len(20)]
    pub paths: Vec<BridgePath>,
}

impl ChainBridgePaths {
    pub fn is_path_supported(&self, source_mint: &Pubkey, destination_token: &[u8; 32]) -> bool {
        self.paths.iter().any(|p|
            p.source_mint == *source_mint && p.destination_token == *destination_token
        )
    }

    pub fn size(num_paths: usize) -> usize {
        8 + 1 + 4 + 4 + (num_paths * BridgePath::SIZE)
    }
}
```

### BridgePath

```rust
#[derive(Clone, AnchorSerialize, AnchorDeserialize, InitSpace, PartialEq)]
pub struct BridgePath {
    /// Extension mint on Solana (e.g., wM mint pubkey)
    pub source_mint: Pubkey,         // 32 bytes
    /// Token address on destination chain (e.g., Ethereum wM address)
    pub destination_token: [u8; 32], // 32 bytes
}

impl BridgePath {
    pub const SIZE: usize = 64;
}
```

## New Instructions

### initialize_chain_paths

Creates a new ChainBridgePaths PDA for a destination chain.

**Accounts:**

- `admin` (signer, mut) - Must match portal_global.admin
- `portal_global` - Validates admin
- `chain_paths` (init) - New PDA to create
- `system_program`

**Args:**

- `destination_chain_id: u32`

**Constraints:**

- Admin only
- Chain paths PDA must not already exist

### add_bridge_path

Adds a new source→destination token path to a chain's configuration.

**Accounts:**

- `admin` (signer, mut)
- `portal_global` - Validates admin
- `chain_paths` (mut, realloc) - Grows by 64 bytes
- `system_program`

**Args:**

- `destination_chain_id: u32`
- `source_mint: Pubkey`
- `destination_token: [u8; 32]`

**Constraints:**

- Admin only
- Path must not already exist (no duplicates)

### remove_bridge_path

Removes a source→destination token path from a chain's configuration.

**Accounts:**

- `admin` (signer, mut)
- `portal_global` - Validates admin
- `chain_paths` (mut, realloc) - Shrinks by 64 bytes
- `system_program`

**Args:**

- `destination_chain_id: u32`
- `source_mint: Pubkey`
- `destination_token: [u8; 32]`

**Constraints:**

- Admin only
- Path must exist

## Modified Instructions

### send_token

**New Account:**

```rust
/// Chain-specific bridge path configuration
#[account(
    seeds = [CHAIN_PATHS_SEED, destination_chain_id.to_be_bytes().as_ref()],
    bump = chain_paths.bump,
)]
pub chain_paths: Account<'info, ChainBridgePaths>,
```

**New Validation:**

```rust
// In validate() function, after existing checks:
if !self.chain_paths.is_path_supported(
    &self.extension_mint.key(),
    &destination_token,
) {
    return err!(BridgeError::UnsupportedBridgePath);
}
```

## New Error Codes

Add to `packages/common/src/errors.rs`:

```rust
#[msg("Bridge path is not supported")]
UnsupportedBridgePath,

#[msg("Bridge path already exists")]
PathAlreadyExists,

#[msg("Bridge path not found")]
PathNotFound,
```

## Key Files to Modify

| File                                              | Changes                                                           |
| ------------------------------------------------- | ----------------------------------------------------------------- |
| `programs/portal/src/state.rs`                    | Add `ChainBridgePaths`, `BridgePath`, `CHAIN_PATHS_SEED` constant |
| `programs/portal/src/lib.rs`                      | Add 3 instruction entry points                                    |
| `programs/portal/src/instructions/mod.rs`         | Export `bridge_path` module                                       |
| `programs/portal/src/instructions/bridge_path.rs` | **NEW** - 3 instruction handlers                                  |
| `programs/portal/src/instructions/send_token.rs`  | Add `chain_paths` account, update validation                      |
| `packages/common/src/errors.rs`                   | Add 3 error codes                                                 |

## Implementation Phases

### Phase 1: Core Types and Errors

- [ ] Add `CHAIN_PATHS_SEED` constant to `state.rs`
- [ ] Add `BridgePath` struct to `state.rs`
- [ ] Add `ChainBridgePaths` account struct with `is_path_supported()` and `size()` methods
- [ ] Add `UnsupportedBridgePath`, `PathAlreadyExists`, `PathNotFound` errors to common errors

### Phase 2: Admin Instructions

- [ ] Create `programs/portal/src/instructions/bridge_path.rs`
- [ ] Implement `InitializeChainPaths` instruction
- [ ] Implement `AddBridgePath` instruction with realloc
- [ ] Implement `RemoveBridgePath` instruction with realloc
- [ ] Export module in `instructions/mod.rs`
- [ ] Add entry points in `lib.rs`

### Phase 3: Send Token Integration

- [ ] Add `chain_paths` account to `SendToken` accounts struct
- [ ] Add path validation in `SendToken::validate()`
- [ ] Update any client code / IDL consumers

### Phase 4: Testing

- [ ] Unit test: `ChainBridgePaths::is_path_supported()` helper
- [ ] Integration test: `initialize_chain_paths` success
- [ ] Integration test: `initialize_chain_paths` unauthorized (non-admin)
- [ ] Integration test: `add_bridge_path` success
- [ ] Integration test: `add_bridge_path` duplicate rejected
- [ ] Integration test: `remove_bridge_path` success
- [ ] Integration test: `remove_bridge_path` not found
- [ ] Integration test: `send_token` fails without chain paths initialized
- [ ] Integration test: `send_token` fails with unsupported path
- [ ] Integration test: `send_token` succeeds with valid path

## Verification Plan

### Build Verification

```bash
make build              # Compile all programs
make unit-tests         # Run unit tests
make test               # Run integration tests
```

### Manual Testing on Localnet

1. Start localnet: `make localnet`
2. Initialize portal with admin
3. `initialize_chain_paths(destination_chain_id=1)` for Ethereum
4. `add_bridge_path(1, wM_mint, eth_wM_address)` - add wM→Eth wM path
5. Verify `send_token` from wM to Eth wM succeeds
6. Verify `send_token` from yM to Eth wM fails (UnsupportedBridgePath)
7. `add_bridge_path(1, yM_mint, eth_yM_address)` - add yM→Eth yM path
8. Verify `send_token` from yM to Eth yM succeeds
9. Verify `send_token` from yM to Eth wM still fails (cross-issuer blocked)
10. `remove_bridge_path(1, wM_mint, eth_wM_address)`
11. Verify `send_token` from wM to Eth wM now fails

### Security Review Checklist

- [ ] Only admin can modify paths (has_one constraint)
- [ ] PDA seeds are deterministic and non-colliding
- [ ] Realloc payer is admin (no rent extraction)
- [ ] No duplicate paths allowed
- [ ] Path removal properly shrinks account

## References

- EVM M Portal Lite: `supportedBridgingPath` mapping in `Portal.sol`
- Solana Extensions: ext_swap whitelist pattern in `SwapGlobal`
- Existing portal patterns: `send_token.rs`, `state.rs`
