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
