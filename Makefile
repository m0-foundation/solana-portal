.PHONY: test build test-verbose

test:
	cargo test --package tests -- --test-threads=1

test-verbose:
	cargo test --package tests -- --test-threads=1 --nocapture

build:
	anchor build
	cp -f target/idl/*.json packages/common/idls/
