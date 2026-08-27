#!/usr/bin/env bash
# Optimize the compiled Dongle contract WASM artifact.
#
# Input:  target/wasm32-unknown-unknown/release/dongle_contract.wasm
# Output: target/wasm32-unknown-unknown/release/dongle_contract.optimized.wasm
#
# Optimizer selection (first one found wins):
#   1. `stellar contract optimize`  - Stellar CLI, bundles wasm-opt
#   2. `wasm-opt -Oz`               - Binaryen, standalone
#
# Installing the Stellar CLI:
#   Prefer the prebuilt release binary. `cargo install --locked stellar-cli`
#   builds from source and requires a rustc newer than the toolchain pinned in
#   rust-toolchain.toml, so it fails on this repository's pinned toolchain.
#
#     VERSION=27.1.0
#     curl -sSfL "https://github.com/stellar/stellar-cli/releases/download/v${VERSION}/stellar-cli-${VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
#       | tar -xz -C /usr/local/bin stellar
#
#   Or install Binaryen instead (`apt-get install binaryen`, `brew install binaryen`).
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

if command -v stellar >/dev/null 2>&1; then
  echo "Optimizer: stellar contract optimize ($(stellar --version 2>/dev/null | head -1))"
  stellar contract optimize --wasm "$INPUT" --wasm-out "$OUTPUT"
elif command -v wasm-opt >/dev/null 2>&1; then
  echo "Optimizer: wasm-opt ($(wasm-opt --version 2>/dev/null | head -1))"
  wasm-opt -Oz --enable-bulk-memory --enable-mutable-globals "$INPUT" -o "$OUTPUT"
else
  echo "Error: no WASM optimizer found on PATH (looked for 'stellar' and 'wasm-opt')." >&2
  echo "Install one of:" >&2
  echo "  Stellar CLI (prebuilt): https://github.com/stellar/stellar-cli/releases" >&2
  echo "  Binaryen:               apt-get install binaryen | brew install binaryen" >&2
  exit 127
fi

if [ ! -f "$OUTPUT" ]; then
  echo "Error: optimizer did not produce '$OUTPUT'" >&2
  exit 1
fi

AFTER=$(wc -c < "$OUTPUT")
SAVED=$(( BEFORE - AFTER ))

echo "Original:  ${BEFORE} bytes"
echo "Optimized: ${AFTER} bytes  (saved ${SAVED} bytes)"
echo "✓ Optimized WASM written to $OUTPUT"
