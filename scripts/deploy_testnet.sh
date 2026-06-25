#!/usr/bin/env bash
set -euo pipefail

# Scripts directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Load environment variables if .env file exists
if [ -f "$PROJECT_ROOT/.env" ]; then
    echo "Loading environment variables from .env..."
    # Export vars, ignoring comments
    export $(grep -v '^#' "$PROJECT_ROOT/.env" | xargs)
fi

# Required environment variables with defaults
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-}"
NETWORK="${NETWORK:-testnet}"
RPC_URL="${RPC_URL:-https://soroban-testnet.stellar.org:443}"
PASSPHRASE="${PASSPHRASE:-Test SDF Network ; September 2015}"

if [ -z "$DEPLOYER_IDENTITY" ]; then
    echo "Error: DEPLOYER_IDENTITY environment variable is not set."
    echo "Please set it (e.g. export DEPLOYER_IDENTITY=alice) or define it in a .env file."
    exit 1
fi

echo "=== Deployment Configuration ==="
echo "Identity:   $DEPLOYER_IDENTITY"
echo "Network:    $NETWORK"
echo "RPC URL:    $RPC_URL"
echo "================================="

# Ensure soroban-cli is installed
if ! command -v soroban &> /dev/null; then
    echo "Error: soroban CLI is not installed. Please install it first."
    exit 1
fi

# Configure network in Soroban CLI
echo "Configuring network '$NETWORK'..."
soroban network add \
  --global "$NETWORK" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$PASSPHRASE" || true

# Ensure deployer identity exists, otherwise generate it
if ! soroban keys address "$DEPLOYER_IDENTITY" &> /dev/null; then
    echo "Identity '$DEPLOYER_IDENTITY' not found. Generating it..."
    soroban keys generate --global "$DEPLOYER_IDENTITY" --network "$NETWORK"
    echo "Identity generated. Funding account..."
    soroban keys fund "$DEPLOYER_IDENTITY" --network "$NETWORK"
fi

DEPLOYER_ADDRESS=$(soroban keys address "$DEPLOYER_IDENTITY")
echo "Deployer Address: $DEPLOYER_ADDRESS"

# Build contract
echo "Building contract..."
cd "$PROJECT_ROOT/dongle-smartcontract"
cargo build --target wasm32-unknown-unknown --release
soroban contract build

# Optimize contract
WASM_PATH="target/wasm32-unknown-unknown/release/dongle_contract.wasm"
OPTIMIZED_WASM_PATH="target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm"

echo "Optimizing contract WASM..."
soroban contract optimize --wasm "$WASM_PATH" --output-dir "target/wasm32-unknown-unknown/release"
# soroban contract optimize places the optimized build in optimized.wasm format or similar.
# Let's verify if the file was created.
if [ -f "target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm" ]; then
    WASM_PATH="target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm"
elif [ -f "target/wasm32-unknown-unknown/release/dongle_contract_optimized.wasm" ]; then
    WASM_PATH="target/wasm32-unknown-unknown/release/dongle_contract_optimized.wasm"
fi

# Deploy
echo "Deploying contract to $NETWORK..."
CONTRACT_ID=$(soroban contract deploy \
  --wasm "$WASM_PATH" \
  --source "$DEPLOYER_IDENTITY" \
  --network "$NETWORK")

echo "✓ Contract successfully deployed!"
echo "Contract ID: $CONTRACT_ID"

# Save contract ID for subsequent scripts
echo "$CONTRACT_ID" > "$PROJECT_ROOT/.contract_id"
echo "Contract ID saved to $PROJECT_ROOT/.contract_id"
