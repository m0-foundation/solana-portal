.PHONY: test build test-verbose send_token_svm send_token_evm send_index_svm send_index_evm docker-build docker-push

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

# Usage: make send_token_svm AMOUNT=1000 ADAPTER=wormhole
send_token_svm:
	export DEVNET_RPC_URL=$$(op read "op://Solana Dev/Helius/dev rpc") \
	cd cli && cargo run send-token $(AMOUNT) 11155111 0x12b1A4226ba7D9Ad492779c924b0fC00BDCb6217 --adapter $(ADAPTER)

# Usage: make send_token_evm AMOUNT=1000 ADAPTER=wormhole
send_token_evm:
	export SEPOLIA_RPC_URL=$$(op read "op://Solana Dev/Alchemy/sepolia") \
	export PRIVATE_KEY=$$(op read "op://Solana Dev/Ethereum Test Wallet/Wallet/key") \
	cd cli && cargo run send-evm-token $(AMOUNT) D76ySoHPwD8U2nnTTDqXeUJQg5UkD9UD1PUE1rnvPAGm --adapter $(ADAPTER)

# Usage: make send_index_svm CHAIN_ID=1 ADAPTER=wormhole
send_index_svm:
	export MAINNET_RPC_URL=$$(op read "op://Solana Dev/Helius/prod rpc") \
	cd cli && cargo run send-index $(CHAIN_ID) --adapter $(ADAPTER) --network mainnet

# Usage: make send_index_evm ADAPTER=wormhole
send_index_evm:
	export EVM_RPC_URL=$$(op read "op://Solana Dev/Alchemy/mainnet") \
	export EVM_KEY=$$(op read "op://Solana Dev/Ethereum Test Wallet/Wallet/key") \
	cd cli && cargo run send-evm-index --adapter $(ADAPTER) --network mainnet

# service for pushing index updates
push-index-update-image:
	docker build -f cli/Dockerfile -t ghcr.io/m0-foundation/solana-portal:cli --platform linux/amd64 .
	docker push ghcr.io/m0-foundation/solana-portal:cli
