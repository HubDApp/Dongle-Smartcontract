#!/usr/bin/env bash
#
# verify-contract-interface.sh
#
# Checks that docs/CONTRACT_INTERFACE.md documents every public entry point of
# DongleContract and does not document functions that no longer exist.
#
# Exit codes:
#   0  every `pub fn` in lib.rs has a matching `### `fn`` heading, and vice versa
#   1  coverage gap (missing or stale documentation)
#
# Usage:  ./scripts/verify-contract-interface.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT/dongle-smartcontract/src/lib.rs"
DOC="$ROOT/docs/CONTRACT_INTERFACE.md"

[ -f "$LIB" ] || { echo "missing $LIB" >&2; exit 2; }
[ -f "$DOC" ] || { echo "missing $DOC" >&2; exit 2; }

# Public functions declared in the contract impl block.
grep -oE '^\s*pub fn [a-z_0-9]+' "$LIB" \
  | sed -E 's/.*pub fn //' \
  | sort -u > /tmp/dci_actual.txt

# Functions with a dedicated section in the interface doc.
grep -oE '^### `[a-z_0-9]+`' "$DOC" \
  | tr -d '#` ' \
  | sort -u > /tmp/dci_documented.txt

missing="$(comm -23 /tmp/dci_actual.txt /tmp/dci_documented.txt || true)"
stale="$(comm -13 /tmp/dci_actual.txt /tmp/dci_documented.txt || true)"

status=0

if [ -n "$missing" ]; then
  echo "❌ public functions with NO section in CONTRACT_INTERFACE.md:"
  echo "$missing" | sed 's/^/   - /'
  status=1
fi

if [ -n "$stale" ]; then
  echo "❌ CONTRACT_INTERFACE.md documents functions not found in lib.rs:"
  echo "$stale" | sed 's/^/   - /'
  status=1
fi

actual_count="$(wc -l < /tmp/dci_actual.txt | tr -d ' ')"
documented_count="$(wc -l < /tmp/dci_documented.txt | tr -d ' ')"

if [ "$status" -eq 0 ]; then
  echo "✅ CONTRACT_INTERFACE.md covers all $actual_count public functions"
else
  echo
  echo "actual: $actual_count   documented: $documented_count"
fi

exit "$status"
