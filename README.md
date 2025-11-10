# Solana Portal

SVM portal with bridge adapters for sending crosschain messages

## Development

Program are deployed and tested using [Surfpool](https://github.com/txtx/surfpool)

### Installation

```
brew tap txtx/taps
brew install surfpool
```

### Build and run locally

```
anchor build
surfpool start
```

### Runbooks

See `txtx.yml` for runbooks and enviroments

Example:

```
surfpool run deployment --env devnet
```

### Tests

Tests run sequentially against a local surfnet validator. The deployment runbook deploys each anchor program to the local validator.

**Best Practice:** Use runbooks for testing to ensure consistent deployment and configuration across devnet and mainnet environments.

**Note:** Rust executes tests alphabetically when run sequentially. Consider this ordering for tests with dependencies, as they all execute against the same local validator instance.

```
cargo test --package tests
```
