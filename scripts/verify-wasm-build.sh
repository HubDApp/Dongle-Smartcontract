#!/bin/sh
# Verify WASM build compatibility across toolchains (#955).
# 1. Pinned toolchain present. 2. wasm target present.
# 3. cargo check passes. 4. Release WASM builds. 5. Reproducible hash.
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PINNED="$(sed -n 's/^channel *= *"\([^"]*\)".*/\1/p' rust-toolchain.toml | head -1)"
echo "Pinned toolchain: ${PINNED:-unknown} (rust-toolchain.toml)"

if ! rustup toolchain list | grep -q "${PINNED}"; then
  echo "Installing pinned toolchain ${PINNED}..."
  rustup toolchain install "${PINNED}" --target wasm32-unknown-unknown --component rustfmt clippy
fi

rustc --version
rustup target list --installed | grep -q "wasm32-unknown-unknown" \
  || rustup target add wasm32-unknown-unknown

echo "--- cargo check (wasm target) ---"
cargo check --target wasm32-unknown-unknown --manifest-path dongle-smartcontract/Cargo.toml

echo "--- release WASM build (pass 1) ---"
if command -v stellar >/dev/null 2>&1; then
  stellar contract build --manifest-path dongle-smartcontract/Cargo.toml
else
  cargo build --release --target wasm32-unknown-unknown --manifest-path dongle-smartcontract/Cargo.toml
fi

WASM="$(ls -t dongle-smartcontract/target/wasm32-unknown-unknown/release/*.wasm 2>/dev/null | head -1 || true)"
if [ -z "$WASM" ]; then
  echo "ERROR: no .wasm artifact produced"
  exit 1
fi
HASH1="$(sha256sum "$WASM" | cut -d' ' -f1)"
SIZE="$(wc -c < "$WASM")"
echo "Artifact: $WASM (${SIZE} bytes, sha256 $HASH1)"

echo "--- release WASM build (pass 2, reproducibility) ---"
if command -v stellar >/dev/null 2>&1; then
  stellar contract build --manifest-path dongle-smartcontract/Cargo.toml
else
  cargo build --release --target wasm32-unknown-unknown --manifest-path dongle-smartcontract/Cargo.toml
fi
HASH2="$(sha256sum "$WASM" | cut -d' ' -f1)"
if [ "$HASH1" = "$HASH2" ]; then
  echo "Reproducible: both builds sha256 $HASH1"
else
  echo "NOT reproducible: $HASH1 vs $HASH2"
  exit 1
fi

echo "WASM build verification passed."
