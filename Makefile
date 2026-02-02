.PHONY: test build test-verbose send_token_svm send_token_evm

test:
	anchor build -- --features skip-validation
	cargo test --package tests -- --test-threads=1

unit-tests:
	cargo test --workspace --exclude tests

test-verbose:
	cargo test --package tests -- --test-threads=1 --nocapture

build:
	anchor build
	cp -f target/idl/*.json packages/common/idls/

build-devnet:
	anchor build -p wormhole_adapter -- --features devnet,legacy-ntt --no-default-features
	anchor build -p portal

build-testnet:
	anchor build -p hyperlane_adapter -- --features testnet --no-default-features
	anchor build -p portal

build-mainnet:
	anchor build --verifiable  -- --features mainnet --no-default-features

localnet:
	surfpool start -r deployment -r initialize -a test4MzZzYk2NAP1222FSuKqq83GuXY5tHakqREDHPo --rpc-url https://hatty-73mn84-fast-mainnet.helius-rpc.com

publish-common:
	cd packages/common && \
	cargo build && \
	cargo publish --allow-dirty

# Usage: make send_token_svm AMOUNT=1000 DEST_CHAIN=1 RECIPIENT=<address> ADAPTER=hyperlane
send_token_svm:
	DEVNET_RPC_URL=$$(op read "op://Solana Dev/Helius/dev rpc") \
	cargo run --package cli -- send-token $(AMOUNT) $(DEST_CHAIN) $(RECIPIENT) --adapter $(ADAPTER)

# Usage: make send_token_evm AMOUNT=1000 RECIPIENT=<address> ADAPTER=hyperlane
send_token_evm:
	SEPOLIA_RPC_URL=$$(op read "op://Solana Dev/Alchemy/sepolia") \
	PRIVATE_KEY=$$(op read "op://Solana Dev/Ethereum Test Wallet/Wallet/key") \
	cargo run --package cli -- send-evm-token $(AMOUNT) $(RECIPIENT) --adapter $(ADAPTER)
