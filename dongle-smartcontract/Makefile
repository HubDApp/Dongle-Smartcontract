# Dongle Smart Contract Makefile

.PHONY: help build test clean deploy-testnet deploy-local fmt lint check

help:
	@echo "Dongle Smart Contract - Available Commands:"
	@echo ""
	@echo "  make build          - Build the contract (optimized)"
	@echo "  make test           - Run all tests"
	@echo "  make test-verbose   - Run tests with output"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make fmt            - Format code"
	@echo "  make lint           - Run clippy linter"
	@echo "  make check          - Check code without building"
	@echo "  make deploy-testnet - Deploy to Stellar testnet"
	@echo "  make deploy-local   - Deploy to local standalone network"
	@echo ""

build:
	@echo "Building contract..."
	@cargo build --target wasm32-unknown-unknown --release
	@soroban contract build
	@echo "✓ Build complete: target/wasm32-unknown-unknown/release/dongle_contract.wasm"

test:
	@echo "Running tests..."
	@cargo test

test-verbose:
	@echo "Running tests with output..."
	@cargo test -- --nocapture

test-release:
	@echo "Running tests in release mode..."
	@cargo test --release

clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@echo "✓ Clean complete"

fmt:
	@echo "Formatting code..."
	@cargo fmt
	@echo "✓ Format complete"

lint:
	@echo "Running clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings

check:
	@echo "Checking code..."
	@cargo check --target wasm32-unknown-unknown

# Deployment targets (requires configuration)
deploy-testnet: build
	@echo "Deploying to testnet..."
	@echo "Make sure you have configured your identity and network!"
	@soroban contract deploy \
		--wasm target/wasm32-unknown-unknown/release/dongle_contract.wasm \
		--source alice \
		--network testnet

deploy-local: build
	@echo "Deploying to local network..."
	@soroban contract deploy \
		--wasm target/wasm32-unknown-unknown/release/dongle_contract.wasm \
		--source alice \
		--network local

# Development workflow
dev: fmt lint test build
	@echo "✓ Development checks complete"

# CI workflow
ci: check lint test
	@echo "✓ CI checks complete"
