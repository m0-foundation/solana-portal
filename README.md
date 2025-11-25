# Solana Portal

SVM portal with bridge adapters for sending crosschain messages

## Structure

```
programs
├── portal
├── hyperlane-adapter
└── wormhole-adapter
packages
├── common
└── common-macros
runbooks
├── deployment
├── initialize
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
cargo test --package tests
```
