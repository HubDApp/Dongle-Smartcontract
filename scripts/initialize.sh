#!/usr/bin/env bash
set -euo pipefail

# Scripts directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Load environment variables if .env file exists
if [ -f "$PROJECT_ROOT/.env" ]; then
    echo "Loading environment variables from .env..."
    export $(grep -v '^#' "$PROJECT_ROOT/.env" | xargs)
fi

# Configuration
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-}"
NETWORK="${NETWORK:-testnet}"
CONTRACT_ID="${CONTRACT_ID:-}"
ADMIN_ADDRESS="${ADMIN_ADDRESS:-}"

# Load contract ID from file if not provided as env var
if [ -z "$CONTRACT_ID" ] && [ -f "$PROJECT_ROOT/.contract_id" ]; then
    CONTRACT_ID=$(cat "$PROJECT_ROOT/.contract_id")
fi

if [ -z "$CONTRACT_ID" ]; then
    echo "Error: CONTRACT_ID is not set. Please deploy the contract first or set CONTRACT_ID."
    exit 1
fi

if [ -z "$DEPLOYER_IDENTITY" ]; then
    echo "Error: DEPLOYER_IDENTITY is not set. Please set DEPLOYER_IDENTITY."
    exit 1
fi

# If ADMIN_ADDRESS is not set, resolve it using the deployer identity
if [ -z "$ADMIN_ADDRESS" ]; then
    if command -v soroban &> /dev/null; then
        ADMIN_ADDRESS=$(soroban keys address "$DEPLOYER_IDENTITY")
    else
        echo "Error: ADMIN_ADDRESS is not set and soroban CLI is not available to resolve identity address."
        exit 1
    fi
fi

echo "=== Initializing Contract ==="
echo "Contract ID:   $CONTRACT_ID"
echo "Identity:      $DEPLOYER_IDENTITY"
echo "Admin Address: $ADMIN_ADDRESS"
echo "Network:       $NETWORK"
echo "============================="

# Invoke initialization
soroban contract invoke \
  --id "$CONTRACT_ID" \
  --source "$DEPLOYER_IDENTITY" \
  --network "$NETWORK" \
  -- initialize \
  --admin "$ADMIN_ADDRESS"

echo "✓ Contract successfully initialized!"
