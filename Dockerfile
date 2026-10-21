# ─────────────────────────────────────────────────────────────────────────────
# Stage 1 — builder
#
# Installs Rust (pinned to the version in rust-toolchain.toml), the WASM
# cross-compilation target, and the Stellar CLI, then builds the contract.
# All tool downloads happen here so the final image stays minimal.
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS builder

# Install OS build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Rust via rustup — version pinned to match rust-toolchain.toml
ENV RUST_VERSION=1.85.0
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path \
        --default-toolchain ${RUST_VERSION} \
        --component rustfmt clippy \
        --target wasm32-unknown-unknown

# Install Stellar CLI (includes Soroban sub-commands).
# Pin to a known-good release for reproducible builds.
ENV STELLAR_CLI_VERSION=22.3.0
RUN cargo install --locked stellar-cli --version ${STELLAR_CLI_VERSION} --features opt

# ── Copy source and build ─────────────────────────────────────────────────────
WORKDIR /build

# Copy workspace manifests first for better layer caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY dongle-smartcontract/Cargo.toml dongle-smartcontract/

# Fetch dependencies before copying source (cache layer)
RUN mkdir -p dongle-smartcontract/src && \
    echo 'fn main() {}' > dongle-smartcontract/src/lib.rs && \
    cargo fetch

# Copy the full source
COPY . .

# Build the WASM contract in release mode
RUN cargo build \
        --release \
        --target wasm32-unknown-unknown \
        --package dongle-contract

# ─────────────────────────────────────────────────────────────────────────────
# Stage 2 — runtime / dev environment
#
# Minimal image that carries:
#   • The compiled WASM artifact
#   • Rust toolchain + WASM target (for running tests / re-building in-container)
#   • Stellar CLI
#   • Deployment scripts
#
# This image is also used by docker-compose for local development.
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="Dongle Smart Contract Dev Environment"
LABEL org.opencontainers.image.description="Soroban/Stellar development environment for the Dongle smart contract"
LABEL org.opencontainers.image.source="https://github.com/HubDApp/Dongle-Smartcontract"

# Runtime OS packages
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    pkg-config \
    libssl-dev \
    build-essential \
    bash \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Copy Rust toolchain and Cargo registry from builder
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:$PATH

COPY --from=builder /usr/local/rustup /usr/local/rustup
COPY --from=builder /usr/local/cargo  /usr/local/cargo

# Copy compiled WASM artifact
WORKDIR /app
COPY --from=builder \
    /build/target/wasm32-unknown-unknown/release/dongle_contract.wasm \
    /app/dongle_contract.wasm

# Copy full project source so developers can rebuild / run tests in-container
COPY . /app

# Expose Stellar standalone node RPC port (used by docker-compose)
EXPOSE 8000

# Default command: run the test suite.
# Override with `docker run ... make build` or any other Make target.
CMD ["cargo", "test", "--manifest-path", "/app/dongle-smartcontract/Cargo.toml"]
