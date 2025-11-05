.PHONY: test build

test:
	cargo test --package tests -- --test-threads=1

build:
	anchor build
