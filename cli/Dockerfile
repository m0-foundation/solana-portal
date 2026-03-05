# Stage 1: Build
FROM rust:1.91-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY cli/ cli/
COPY packages/ packages/
COPY programs/ programs/
COPY tests/ tests/

RUN cargo build --release --package portal-cli

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/portal-cli /usr/local/bin/portal-cli

ENV NETWORK=mainnet

ENTRYPOINT ["portal-cli", "send-evm-index", "--adapter", "wormhole"]
