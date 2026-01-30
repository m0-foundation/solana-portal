# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Solana Portal is a crosschain messaging and bridging system on Solana that acts as a hub receiving messages from different bridge protocols (Wormhole and Hyperlane). It enables secure inter-chain communication for token transfers, order book reports (fill/cancel), and state synchronization (merkle roots, indexes).

## Build & Test Commands

```bash
# Build
make build              # Build all programs with IDL generation
make build-mainnet      # Verifiable mainnet build

# Test
make test               # Integration tests (sequential, shared validator)
make unit-tests         # Unit tests only (excludes integration tests)
make test-verbose       # Integration tests with output

# Local Development
make localnet           # Start Surfpool validator with deployment/initialization
```

**Note:** Integration tests run sequentially (`--test-threads=1`) against a Surfpool validator. Tests execute alphabetically and share state, so test file naming (tests_01, tests_02, etc.) matters.

## Architecture

### Three-Tier System

1. **Portal Program** (`programs/portal/`) - Central hub managing state and coordinating adapters

   - Admin functions: initialize, pause/unpause, admin transfer
   - Outbound: send_token, send_index, send_merkle_root, send_report
   - Inbound: receive_message (dispatches based on adapter)

2. **Bridge Adapters** - Protocol-specific implementations

   - `programs/wormhole-adapter/` - VAA-based verification, lookup tables
   - `programs/hyperlane-adapter/` - ISM verification, IGP gas payments

3. **Common Package** (`packages/common/`) - Shared types, traits, payloads, account validation

### Message Flow

**Sending:** Caller → Portal `send_*` instruction → Constructs PayloadData → CPI to adapter's `send_message` → Bridge-specific transmission

**Receiving:** Relayer → Adapter receives with bridge data → Bridge verification (VAA/ISM) → CPI to Portal `receive_message` → Dispatch by payload type → Replay protection via BridgeMessage account

### Payload Types (in `packages/common/src/payloads.rs`)

- TokenTransferPayload, IndexPayload, MerkleRootPayload
- FillReportPayload, CancelReportPayload (for order book)

## Key Files

- `programs/*/src/lib.rs` - Program entry points with instruction definitions
- `programs/*/src/instructions/mod.rs` - Instruction routing and `send_message` helper
- `packages/common/src/adapter_accounts.rs` - Required accounts for each adapter operation
- `tests/src/lib.rs` - Surfpool validator lifecycle for tests
- `txtx.yml` - Surfpool runbooks and environment configs (localnet/devnet/testnet/mainnet)

## Anchor Patterns

### Program File Structure

Each program follows this structure:

- `lib.rs` - `#[program]` macro with instruction routing
- `state.rs` - Account structs with `#[account]` and `#[derive(InitSpace)]`
- `instructions/*.rs` - Individual instruction handlers
- `consts.rs` - PDA seeds defined as `#[constant]`

### Instruction Pattern

```rust
#[derive(Accounts)]
pub struct InstructionName<'info> {
    #[account(mut, seeds = [SEED], bump)]
    pub account: Account<'info, State>,
}

impl InstructionName<'_> {
    pub fn handler(ctx: Context<Self>, args: Args) -> Result<()> { ... }
}
```

### Common Constraints

- `seeds = [SEED_CONST, key.as_ref()], bump` - PDA derivation
- `has_one = admin` - Verify account relationship
- `constraint = !global.paused @ BridgeError::Paused` - Pause checks
- `/// CHECK:` comments required for `UncheckedAccount`

### CPI with PDA Signer

```rust
let seeds = &[AUTHORITY_SEED, &[ctx.accounts.global.bump]];
let signer_seeds = &[&seeds[..]];
let cpi_ctx = CpiContext::new_with_signer(program, accounts, signer_seeds);
```

## Common Package Utilities

- `declare_program!(name)` - CPI-enabled program declaration
- `pda!(&[seeds], &program_id)` - Compute PDA address
- `#[derive(ExtractAccounts)]` - Extract from remaining_accounts
- `BridgeError` enum in `packages/common/src/errors.rs` - Shared errors
