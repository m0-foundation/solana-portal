# Solana Portal

SVM portal with bridge adapters for sending crosschain messages

## Structure

```
programs
├── portal
├── hyperlane-adapter
└── wormhole-adapter
packages
├── m0-portal-common
└── m0-portal-common-macros
runbooks
├── deployment
├── initialize
├── pause
└── set_peers
tests
```

- `programs` consists of the Portal program and any necessary adapters for sending and receiving messages
- `packages` contains common code between bridge programs and adapters
- `runbooks` contains all surfpool runbooks for program management (see `txtx.yml`)
- `tests` contains a set of sequential tests run agaist a local surfpool validator

## Development

Program are deployed and tested using [Surfpool](https://github.com/txtx/surfpool)

### Installation

```
brew tap txtx/taps
brew install surfpool
```

### Build and run locally

```
make build
make localnet
```

### Runbooks

See `txtx.yml` for runbooks and enviroments

Example:

```
surfpool run set_peers --env devnet
```

### Tests

Tests run sequentially against a local surfnet validator. The deployment runbook deploys each anchor program to the local validator.

**Best Practice:** Use runbooks for testing to ensure consistent deployment and configuration across devnet and mainnet environments.

**Note:** Rust executes tests alphabetically when run sequentially. Consider this ordering for tests with dependencies, as they all execute against the same local validator instance.

```
make test
```

## Bridge Flows

### Sending a bridge message

Each bridge message has its own instruction that can be called that will construct a payload and send it using the provided bridge adapter. All instructions used for sending messages contain an interface account `bridge_adapter` used to specifiy which bridge adapter wil be used. This determines which remaining accounts are required for [send_message](programs/portal/src/instructions/mod.rs). There are helper methods in [adapter_accounts.rs](packages/common/src/adapter_accounts.rs) to get the required account metas each supported bridge. See [tests_04_index_updates.rs](tests/src/tests_04_index_update.rs) for an example of sending a bridge message to each supported bridge adapter.

Note that chain_ids differ depending on the adapter being used so make sure to specify the correct one. Bridge adapters have a list of supported peers based on chain_id and will reject invalid requests.

### Receiving a bridge message

When a bridge adapter receives a bridge message it will relay it to [receive_message](programs/portal/src/instructions/receive_message.rs) on the Portal. Adapters typically have some resolver instruction used by relayers to provide the accounts required to call the receive message instruction. The accounts required will differ depending on the bridge adapter and the message being relayed. Each payload has a [parse_and_validate_accounts](packages/common/src/accounts.rs) used to extract the required remaining accounts for the receive instrunction.

### Adding a new bridge adapter

Bridge adapters only need a `receive_message` and `send_message` instruction. All other instructions added are there to support those methods. Helpers and interfaces in `packages/common` will need to be updated with the new adapter along with [send_message](programs/portal/src/instructions/mod.rs).
