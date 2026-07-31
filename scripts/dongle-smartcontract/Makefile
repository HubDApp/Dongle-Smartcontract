# Dongle Smart Contract Makefile

.PHONY: help build test test-verbose test-release clean fmt lint check deploy-testnet deploy-local dev ci

help:
	@echo "Dongle Smart Contract - Available Commands:"
	@echo ""
	@echo "  make build          - Build the contract (WASM)"
	@echo "  make test           - Run all tests"
	@echo "  make test-verbose   - Run tests with output"
	@echo "  make test-release   - Run tests in release mode"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make fmt            - Format code"
	@echo "  make lint           - Run clippy linter"
	@echo "  make check          - Check code without building"
	@echo "  make deploy-testnet - Deploy to Stellar testnet (requires DEPLOYER_IDENTITY)"
	@echo "  make deploy-local   - Deploy to local standalone network (requires DEPLOYER_IDENTITY)"
	@echo ""
	@echo "Deployment environment variables:"
	@echo "  DEPLOYER_IDENTITY   - Stellar identity/key name to use for deployment (required)"
	@echo ""

# DEPLOYER_IDENTITY must be set before running deploy targets.
# Example: make deploy-testnet DEPLOYER_IDENTITY=my-key
DEPLOYER_IDENTITY ?=

build:
	@echo "Building contract..."
	@stellar contract build
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

# Deployment targets — set DEPLOYER_IDENTITY before running.
# Example: make deploy-testnet DEPLOYER_IDENTITY=alice
deploy-testnet: build
	@if [ -z "$(DEPLOYER_IDENTITY)" ]; then \
		echo "Error: DEPLOYER_IDENTITY is not set. Usage: make deploy-testnet DEPLOYER_IDENTITY=<key-name>"; \
		exit 1; \
	fi
	@echo "Deploying to testnet (identity: $(DEPLOYER_IDENTITY))..."
	@stellar contract deploy \
		--wasm target/wasm32-unknown-unknown/release/dongle_contract.wasm \
		--source $(DEPLOYER_IDENTITY) \
		--network testnet

deploy-local: build
	@if [ -z "$(DEPLOYER_IDENTITY)" ]; then \
		echo "Error: DEPLOYER_IDENTITY is not set. Usage: make deploy-local DEPLOYER_IDENTITY=<key-name>"; \
		exit 1; \
	fi
	@echo "Deploying to local network (identity: $(DEPLOYER_IDENTITY))..."
	@stellar contract deploy \
		--wasm target/wasm32-unknown-unknown/release/dongle_contract.wasm \
		--source $(DEPLOYER_IDENTITY) \
		--network local

# Development workflow
dev: fmt lint test build
	@echo "✓ Development checks complete"

# CI workflow
ci: check lint test
	@echo "✓ CI checks complete"
