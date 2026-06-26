#!/usr/bin/env bash
# Optimize the compiled Dongle contract WASM artifact using the Stellar CLI.
#
# Input:  target/wasm32-unknown-unknown/release/dongle_contract.wasm
# Output: target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm
#
# Prerequisites:
#   cargo install --locked stellar-cli --features opt
#   The --features opt flag bundles wasm-opt into the stellar binary.
#
# Usage:
#   ./scripts/optimize_wasm.sh [INPUT_WASM] [OUTPUT_WASM]
#
# Both arguments are optional; defaults point to the workspace release directory.

set -euo pipefail

INPUT="${1:-target/wasm32-unknown-unknown/release/dongle_contract.wasm}"
OUTPUT="${2:-target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm}"

if [ ! -f "$INPUT" ]; then
  echo "Error: input WASM not found at '$INPUT'" >&2
  echo "Build it first:" >&2
  echo "  cargo build -p dongle-contract --target wasm32-unknown-unknown --release" >&2
  exit 1
fi

BEFORE=$(wc -c < "$INPUT")

echo "Optimizing $INPUT -> $OUTPUT"
stellar contract optimize --wasm "$INPUT" --wasm-out "$OUTPUT"

AFTER=$(wc -c < "$OUTPUT")
SAVED=$(( BEFORE - AFTER ))

echo "Original:  ${BEFORE} bytes"
echo "Optimized: ${AFTER} bytes  (saved ${SAVED} bytes)"
echo "✓ Optimized WASM written to $OUTPUT"
