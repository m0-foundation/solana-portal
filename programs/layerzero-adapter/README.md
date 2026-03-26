# LayerZero Adapter

Bridge adapter that connects the M0 Portal to the [LayerZero V2](https://layerzero.network/) messaging network on Solana. Enables cross-chain communication for token transfers, index propagation, merkle root updates, and order book reports via LayerZero.

**Program ID:** `MzLzScr2JSzmxfDfg38ZPsw7RhRUGwkJtr2whLo7uru`

## Architecture

The adapter sits between the Portal (M0's central hub) and the LayerZero Endpoint program, translating between M0's payload format and LayerZero's messaging protocol.

```
Outbound:  Portal ──CPI──> LZ Adapter ──invoke_signed──> LZ Endpoint ──> Remote Chain
Inbound:   Remote Chain ──> LZ Executor ──> LZ Adapter ──CPI──> Portal
```

### OApp Registration

The adapter registers as a LayerZero OApp during initialization. The `lz_global` PDA (`seeds=[b"global"]`) acts as the OApp identity — it is the address registered with the LayerZero Endpoint and signs all endpoint CPIs.

### Signing PDAs

Two PDAs serve distinct roles:

| PDA                    | Seeds            | Signs                   | Purpose                                                          |
| ---------------------- | ---------------- | ----------------------- | ---------------------------------------------------------------- |
| `lz_global`            | `[b"global"]`    | LayerZero Endpoint CPIs | OApp identity: send, clear, register_oapp, set_delegate          |
| `lz_adapter_authority` | `[b"authority"]` | Portal CPIs             | Proves to Portal that the message came from a registered adapter |

### Endpoint Integration

All LayerZero Endpoint interactions use raw `invoke_signed` with manually constructed instructions. The adapter does not depend on the LayerZero `oapp` crate (which has incompatible Anchor version and transitive path dependencies to the endpoint program). Anchor instruction discriminators are pre-computed as constants in `consts.rs` with a unit test verifying correctness.

## Instructions

### Admin

| Instruction                                                | Description                                                                                                                                                       |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `initialize`                                               | Creates `LayerZeroGlobal` account and registers as OApp with the LZ Endpoint. Endpoint accounts (oapp_registry, event_authority) passed via `remaining_accounts`. |
| `set_peer`                                                 | Registers a remote chain peer mapping M0 chain ID to LZ Endpoint ID (EID) and remote peer address. Uses `realloc` to grow the account as peers are added.         |
| `set_delegate`                                             | Changes the OApp's delegate on the LZ Endpoint. The delegate can configure messaging libraries and other endpoint settings.                                       |
| `pause_outgoing` / `unpause_outgoing`                      | Pauses/unpauses outbound message sending.                                                                                                                         |
| `pause_incoming` / `unpause_incoming`                      | Pauses/unpauses inbound message processing.                                                                                                                       |
| `propose_admin` / `accept_admin` / `cancel_admin_transfer` | Two-step admin transfer.                                                                                                                                          |

### Outbound

| Instruction    | Description                                                                                                                                                                                                                                                                                                          |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `send_message` | Called by Portal via CPI. Looks up the peer for the destination chain, constructs an M0 `Payload` (header + data), and forwards it to the LZ Endpoint `send` instruction. Portal authority must be a signer, proving the call originates from Portal. LZ Endpoint send accounts are passed via `remaining_accounts`. |
| `quote`        | Wraps the LZ Endpoint `quote` instruction to estimate messaging fees. Returns `MessagingFee { native_fee, lz_token_fee }` via `set_return_data`.                                                                                                                                                                     |

### Inbound

| Instruction        | Description                                                                                                                                                                                                                                                                                                                                                                              |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `lz_receive`       | Called by the LayerZero Executor. Validates the sender against the registered peer for the source EID. Calls `clear()` on the LZ Endpoint (burns the nonce for replay protection). Decodes the M0 payload and CPIs to Portal's `receive_message`.                                                                                                                                        |
| `lz_receive_types` | Simulation/view instruction called by the LZ Executor to discover the accounts needed for `lz_receive`. Reads `EarnGlobal` (for `m_mint`) and `SwapGlobal` (for whitelisted extensions) from `remaining_accounts`, then calls `require_metas()` to resolve payload-specific accounts (user ATAs, extension vaults, order PDAs). Returns the complete account list via `set_return_data`. |

## Message Flows

### Sending (Portal to Remote Chain)

1. A caller invokes a Portal send instruction (`send_token`, `send_index`, etc.)
2. Portal constructs a `PayloadData` and calls `send_message` on the adapter via CPI, signing with its `portal_authority` PDA
3. The adapter looks up the peer for the destination M0 chain ID to get the LZ EID and remote peer address
4. The adapter wraps the payload in an M0 `Payload` (header with message_id, destination, payload_type, index + data)
5. The adapter calls LZ Endpoint `send` via `invoke_signed`, with `lz_global` signing as the OApp
6. LayerZero's off-chain infrastructure (DVNs, Executor) delivers the message to the remote chain

### Receiving (Remote Chain to Portal)

1. A message arrives from a remote chain via LayerZero
2. The LZ Executor calls `lz_receive_types` (simulation) to discover required accounts, passing `EarnGlobal` and `SwapGlobal` in `remaining_accounts`
3. The instruction reads on-chain state, decodes the payload, and returns the full account list including payload-specific accounts (user token ATAs, extension accounts, etc.)
4. The LZ Executor constructs the transaction and calls `lz_receive`
5. The adapter validates the sender matches the registered peer for the source EID
6. The adapter calls `clear()` on the LZ Endpoint — this burns the nonce, providing LayerZero-level replay protection
7. The adapter decodes the M0 `Payload` from the message
8. The adapter CPIs to Portal's `receive_message`, signing with `lz_adapter_authority`
9. Portal verifies the adapter authority, creates a `BridgeMessage` PDA (Portal-level replay protection via `message_id` uniqueness), and dispatches based on payload type

### Replay Protection

Two independent layers:

- **LayerZero level:** The `clear()` call burns the `(receiver, src_eid, sender, nonce)` tuple on the endpoint. The nonce can never be reused.
- **Portal level:** Portal's `receive_message` uses `init` on a `BridgeMessage` PDA derived from `[b"message", message_id]`. Anchor's `init` constraint fails if the account already exists, making each `message_id` one-time across all adapters.

## Account State

### LayerZeroGlobal

```
seeds = [b"global"]
```

| Field              | Type             | Description                                                         |
| ------------------ | ---------------- | ------------------------------------------------------------------- |
| `bump`             | `u8`             | PDA bump seed                                                       |
| `admin`            | `Pubkey`         | Admin authority                                                     |
| `pending_admin`    | `Option<Pubkey>` | Pending admin for two-step transfer                                 |
| `chain_id`         | `u32`            | M0 chain ID for this deployment                                     |
| `endpoint_program` | `Pubkey`         | LZ Endpoint program ID                                              |
| `outgoing_paused`  | `bool`           | Whether outbound sends are paused                                   |
| `incoming_paused`  | `bool`           | Whether inbound receives are paused                                 |
| `peers`            | `Peers`          | Registry of remote chain peers (M0 chain ID <-> LZ EID <-> address) |
| `padding`          | `[u8; 128]`      | Reserved for future fields                                          |

## Peer Registry

Each peer maps between two chain ID systems:

| Field              | Description                                                       |
| ------------------ | ----------------------------------------------------------------- |
| `m0_chain_id`      | Portal's internal chain identifier                                |
| `adapter_chain_id` | LayerZero Endpoint ID (EID) for the remote chain                  |
| `address`          | 32-byte address of the remote peer (the remote adapter or Portal) |

Outbound lookups use `m0_chain_id` (Portal tells the adapter which M0 chain to send to). Inbound lookups use `adapter_chain_id` (the LZ Executor provides the source EID).

## Dependencies

- **No LayerZero SDK crate** — endpoint CPIs use raw `invoke_signed` with Anchor discriminators
- `m0-portal-common` — shared types (Payload, Peers, BridgeError), Portal/Earn/ExtSwap CPI bindings
- `anchor-lang` 0.31.1, `anchor-spl` 0.31.1

## Configuration

The LZ Endpoint program ID is the same on both mainnet and devnet: `76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6`

The Solana EID is 30168 (mainnet) / 40168 (testnet).
