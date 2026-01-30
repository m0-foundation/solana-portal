.PHONY: test build test-verbose

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
	anchor build -p wormhole_adapter -- --features devnet --no-default-features
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
