#!/usr/bin/env python3
"""Validate deployments.json schema and optionally verify on-chain WASM hashes.

Usage
-----
Schema-only (default, no network required):
    python3 scripts/validate_deployments.py

On-chain verification (fetches WASM hash from Stellar RPC nodes):
    python3 scripts/validate_deployments.py --verify-onchain

    Specific network only:
    python3 scripts/validate_deployments.py --verify-onchain --network testnet

    Custom RPC endpoints:
    python3 scripts/validate_deployments.py --verify-onchain \
        --rpc-testnet https://soroban-testnet.stellar.org:443 \
        --rpc-mainnet https://mainnet.stellar.validationcloud.io/v1/<KEY>

Exit codes
----------
0 — all checks passed
1 — validation errors found
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request


# ---------------------------------------------------------------------------
# RPC defaults
# ---------------------------------------------------------------------------

DEFAULT_RPC_URLS: dict[str, str] = {
    "testnet": "https://soroban-testnet.stellar.org",
    "mainnet": "https://mainnet.sorobanrpc.com",
}

# ---------------------------------------------------------------------------
# Regex patterns (mirror deployments.schema.json)
# ---------------------------------------------------------------------------

CONTRACT_ID_RE = re.compile(r"^C[A-Z2-7]{55}$")
WASM_HASH_RE = re.compile(r"^[a-fA-F0-9]{64}$")
GIT_COMMIT_RE = re.compile(r"^[a-f0-9]{7,40}$")
GIT_TAG_RE = re.compile(r"^[a-zA-Z0-9._-]+$")
DEPLOYER_RE = re.compile(r"^[GC][A-Z2-7]{55}$")
TIMESTAMP_RE = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})$"
)


# ---------------------------------------------------------------------------
# Schema validation
# ---------------------------------------------------------------------------

def validate_schema(data: dict) -> list[str]:
    """Validate the structure and field formats of a parsed deployments.json.

    Returns a (possibly empty) list of human-readable error strings.
    """
    errors: list[str] = []

    if not isinstance(data, dict):
        return ["Root of manifest must be a JSON object."]

    allowed_keys = {"$schema", "testnet", "mainnet"}
    required_keys = {"testnet", "mainnet"}

    actual_keys = set(data.keys())
    missing = required_keys - actual_keys
    if missing:
        errors.append(f"Missing required top-level keys: {missing}")

    extra = actual_keys - allowed_keys
    if extra:
        errors.append(f"Unexpected top-level keys: {extra}")

    for network in ["testnet", "mainnet"]:
        if network not in data:
            continue
        entries = data[network]
        if not isinstance(entries, list):
            errors.append(f"Network '{network}' must be an array of deployment entries.")
            continue

        for idx, entry in enumerate(entries):
            loc = f"{network}[{idx}]"
            if not isinstance(entry, dict):
                errors.append(f"Entry {loc} must be a JSON object.")
                continue

            entry_keys = set(entry.keys())
            req_fields = {"contract_id", "wasm_hash", "git_commit", "deployer", "timestamp"}
            opt_fields = {"git_tag"}

            missing_fields = req_fields - entry_keys
            if missing_fields:
                errors.append(f"Entry {loc} is missing required fields: {missing_fields}")

            extra_fields = entry_keys - req_fields - opt_fields
            if extra_fields:
                errors.append(f"Entry {loc} has unexpected fields: {extra_fields}")

            if "contract_id" in entry:
                cid = entry["contract_id"]
                if not isinstance(cid, str) or not CONTRACT_ID_RE.match(cid):
                    errors.append(
                        f"Entry {loc}.contract_id ('{cid}') is invalid. "
                        "Must be a 56-character Stellar Contract ID starting with 'C'."
                    )

            if "wasm_hash" in entry:
                whash = entry["wasm_hash"]
                if not isinstance(whash, str) or not WASM_HASH_RE.match(whash):
                    errors.append(
                        f"Entry {loc}.wasm_hash ('{whash}') is invalid. "
                        "Must be a 64-character hex string."
                    )

            if "git_commit" in entry:
                gc = entry["git_commit"]
                if not isinstance(gc, str) or not GIT_COMMIT_RE.match(gc):
                    errors.append(
                        f"Entry {loc}.git_commit ('{gc}') is invalid. "
                        "Must be a git commit hash (7-40 hex characters)."
                    )

            if "git_tag" in entry:
                gt = entry["git_tag"]
                if not isinstance(gt, str) or not GIT_TAG_RE.match(gt):
                    errors.append(
                        f"Entry {loc}.git_tag ('{gt}') is invalid. "
                        "Must be a valid git tag (alphanumeric, dots, underscores, hyphens)."
                    )

            if "deployer" in entry:
                deployer = entry["deployer"]
                if not isinstance(deployer, str) or not DEPLOYER_RE.match(deployer):
                    errors.append(
                        f"Entry {loc}.deployer ('{deployer}') is invalid. "
                        "Must be a 56-character Stellar account or contract ID starting with 'G' or 'C'."
                    )

            if "timestamp" in entry:
                ts = entry["timestamp"]
                if not isinstance(ts, str) or not TIMESTAMP_RE.match(ts):
                    errors.append(
                        f"Entry {loc}.timestamp ('{ts}') is invalid. "
                        "Must be a valid ISO 8601 / RFC 3339 datetime string (e.g. '2026-06-24T12:00:00Z')."
                    )

    return errors


# ---------------------------------------------------------------------------
# On-chain WASM hash fetching
# ---------------------------------------------------------------------------

def _stellar_rpc_call(rpc_url: str, method: str, params: object, timeout: int = 15) -> object:
    """Send a JSON-RPC 2.0 request to a Stellar RPC node.

    Returns the ``result`` field of the response on success.
    Raises ``RuntimeError`` on HTTP errors, timeouts, or RPC-level errors.
    """
    payload = json.dumps({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    }).encode("utf-8")

    req = urllib.request.Request(
        rpc_url,
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = json.loads(resp.read().decode("utf-8"))
    except urllib.error.URLError as exc:
        raise RuntimeError(f"Network error contacting {rpc_url}: {exc}") from exc

    if "error" in body:
        raise RuntimeError(
            f"RPC error from {rpc_url}: {body['error']}"
        )

    return body.get("result")


def fetch_onchain_wasm_hash(contract_id: str, rpc_url: str) -> str:
    """Return the lowercase hex WASM hash currently deployed for *contract_id*.

    Uses the Stellar RPC ``getLedgerEntries`` method to read the
    ``ContractCode`` ledger entry referenced by the contract instance.

    Raises ``RuntimeError`` if the contract cannot be found or the RPC call
    fails.
    """
    # Step 1: fetch the ContractData (instance) entry to get the WASM hash.
    # The key for a contract instance is an XDR LedgerKey of type CONTRACT_DATA
    # with durability=PERSISTENT and key=LEDGER_ENTRY_DATA (CONTRACT_INSTANCE).
    #
    # Stellar RPC exposes a convenience: passing the contract ID as a
    # StrKey-encoded address to getLedgerEntries with the correct XDR key
    # encoding. We use the simpler `getContractCode` approach available on
    # modern Stellar RPC nodes (horizon-compatible): call `getLedgerEntries`
    # with the contract's instance key.
    #
    # In practice, the cleanest portable approach is to call
    # `getContractWasmByContractId` (available in stellar-cli) or to parse
    # the contract instance XDR. Since we want zero external dependencies we
    # use the `getLedgerEntries` JSON-RPC call with the pre-built XDR key.
    #
    # The Stellar RPC `getLedgerEntries` endpoint accepts a list of
    # base64-encoded XDR `LedgerKey` values.  For a contract instance the key
    # is deterministic from the contract ID; rather than pulling in an XDR
    # library we call the higher-level `getContractData` helper that some RPC
    # nodes expose, falling back to the getLedgerEntries raw approach.

    # Try getContractData (friendbot-compatible RPC nodes, stellar-rpc ≥ 0.0.16)
    try:
        result = _stellar_rpc_call(
            rpc_url,
            "getContractData",
            {
                "contract": contract_id,
                "key": "AAAAAA==",   # ScVal of type VOID — fetches the instance entry
                "durability": "persistent",
            },
        )
        # result.entries[0].val is XDR-encoded ContractDataEntry; we need the
        # wasm_hash from the contract executable inside the instance.
        # Stellar RPC returns the raw XDR, which we cannot parse without a
        # dependency.  Fall through to the getLedgerEntries + wasmHash field.
    except RuntimeError:
        pass

    # Use `getContractCode` which is available on horizon-soroban-rpc nodes
    # and returns the wasmHash directly without XDR parsing.
    result = _stellar_rpc_call(
        rpc_url,
        "getLedgerEntries",
        {"keys": [_contract_instance_ledger_key_xdr(contract_id)]},
    )

    if not result or not result.get("entries"):
        raise RuntimeError(
            f"Contract '{contract_id}' not found on network (RPC: {rpc_url})."
        )

    # The entry's `val` contains the ContractDataEntry XDR.  Stellar RPC also
    # exposes the WASM hash directly in a parsed `wasmHash` field on newer
    # node versions.  Try that first.
    entry = result["entries"][0]

    # Newer RPC nodes include a `wasmHash` convenience field
    if "wasmHash" in entry:
        return entry["wasmHash"].lower()

    # Fall back: the `val` field is base64 XDR.  The WASM hash is a 32-byte
    # value embedded in the instance XDR.  We extract it with a byte-pattern
    # scan — the SHA-256 hash appears as a fixed 32-byte sequence preceded by
    # a 4-byte discriminant for SCO_CONTRACT_EXECUTABLE (type = WASM_REF = 0).
    #
    # This is a best-effort heuristic that works for standard Soroban contracts
    # whose WASM executable is stored inline in the instance entry.
    import base64
    raw = base64.b64decode(entry.get("val", entry.get("xdr", "")))

    # Scan for a 32-byte WASM hash.  In the ContractInstance XDR the
    # executable field encodes as: 4-byte type tag (0x00000000 for WASM) +
    # 32-byte hash.
    wasm_type_tag = b"\x00\x00\x00\x00"
    idx = raw.find(wasm_type_tag)
    if idx != -1 and len(raw) >= idx + 4 + 32:
        candidate = raw[idx + 4: idx + 4 + 32]
        return candidate.hex().lower()

    raise RuntimeError(
        f"Unable to extract WASM hash from ledger entry for contract '{contract_id}'. "
        "The RPC response format may have changed. "
        "Try updating this script or using stellar-cli to verify manually."
    )


def _contract_instance_ledger_key_xdr(contract_id: str) -> str:
    """Return the base64-encoded XDR LedgerKey for a contract's instance entry.

    This constructs the canonical key without any external XDR library by
    encoding the Stellar StrKey-decoded contract ID bytes into the minimal
    LedgerKey XDR structure.

    LedgerKey (CONTRACT_DATA) layout:
        4 bytes  — type discriminant = 0x00000006 (CONTRACT_DATA)
        n bytes  — ScAddress (CONTRACT = 0x00000001 + 32-byte contract hash)
        n bytes  — ScVal key  = VOID (0x00000000, no value)
        4 bytes  — LedgerEntryDurability = 1 (PERSISTENT)
    """
    import base64
    import struct

    # StrKey-decode the contract ID (strip Stellar base32 checksum)
    contract_bytes = _strkey_decode(contract_id)  # 32 bytes

    # Encode as XDR (big-endian, network byte order)
    key_xdr = (
        struct.pack(">I", 6)       # LedgerEntryType = CONTRACT_DATA
        + struct.pack(">I", 1)     # ScAddress type = CONTRACT
        + contract_bytes           # 32-byte contract hash
        + struct.pack(">I", 0)     # ScVal type = VOID (LEDGER_KEY_CONTRACT_INSTANCE)
        + struct.pack(">I", 1)     # LedgerEntryDurability = PERSISTENT
    )

    return base64.b64encode(key_xdr).decode("ascii")


def _strkey_decode(strkey: str) -> bytes:
    """Decode a Stellar StrKey (base32) and return the 32-byte payload.

    Stellar StrKey format:
        1 byte  version (0x08 for Contract = 'C', 0x06 for Account = 'G')
        32 bytes payload
        2 bytes checksum (CRC-16/XModem, little-endian)
    Total base32-encoded: 56 characters.
    """
    import base64

    # Stellar uses a custom base32 alphabet (RFC 4648 without padding)
    alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"

    # Pad to multiple of 8 for standard base64.b32decode
    padded = strkey + "=" * ((8 - len(strkey) % 8) % 8)
    raw = base64.b32decode(padded.upper())

    # raw = 1 (version) + 32 (payload) + 2 (checksum) = 35 bytes
    if len(raw) != 35:
        raise ValueError(f"Invalid StrKey length: expected 35 bytes, got {len(raw)}")

    return raw[1:33]


# ---------------------------------------------------------------------------
# On-chain verification orchestration
# ---------------------------------------------------------------------------

def verify_onchain(data: dict, rpc_urls: dict[str, str]) -> list[str]:
    """For each deployment entry, fetch the on-chain WASM hash and compare.

    Returns a list of error strings (empty = all match).
    """
    errors: list[str] = []

    for network, entries in data.items():
        if network.startswith("$") or not isinstance(entries, list):
            continue

        rpc_url = rpc_urls.get(network)
        if not rpc_url:
            print(f"  [skip] No RPC URL configured for network '{network}', skipping on-chain check.")
            continue

        print(f"  Checking {network} via {rpc_url} ...")

        for idx, entry in enumerate(entries):
            if not isinstance(entry, dict):
                continue

            contract_id = entry.get("contract_id", "")
            expected_hash = entry.get("wasm_hash", "").lower()
            loc = f"{network}[{idx}] ({contract_id})"

            if not contract_id or not expected_hash:
                # Schema errors are already caught by validate_schema; skip here.
                continue

            print(f"    Verifying {loc} ...")

            try:
                onchain_hash = fetch_onchain_wasm_hash(contract_id, rpc_url)
            except RuntimeError as exc:
                errors.append(f"Entry {loc}: failed to fetch on-chain WASM hash — {exc}")
                continue

            if onchain_hash != expected_hash:
                errors.append(
                    f"Entry {loc}: WASM hash MISMATCH — "
                    f"on-chain={onchain_hash!r}, manifest={expected_hash!r}. "
                    "This may indicate an unauthorized contract upgrade."
                )
            else:
                print(f"    ✓ {loc}: WASM hash matches ({onchain_hash[:12]}…)")

    return errors


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate deployments.json and optionally verify on-chain WASM hashes."
    )
    parser.add_argument(
        "--verify-onchain",
        action="store_true",
        default=False,
        help="Fetch each contract's WASM hash from the Stellar RPC node and compare against the manifest.",
    )
    parser.add_argument(
        "--network",
        choices=["testnet", "mainnet"],
        default=None,
        help="Limit on-chain verification to a single network (default: all networks in manifest).",
    )
    parser.add_argument(
        "--rpc-testnet",
        default=DEFAULT_RPC_URLS["testnet"],
        metavar="URL",
        help=f"Stellar RPC URL for testnet (default: {DEFAULT_RPC_URLS['testnet']}).",
    )
    parser.add_argument(
        "--rpc-mainnet",
        default=DEFAULT_RPC_URLS["mainnet"],
        metavar="URL",
        help=f"Stellar RPC URL for mainnet (default: {DEFAULT_RPC_URLS['mainnet']}).",
    )
    parser.add_argument(
        "--manifest",
        default=None,
        metavar="PATH",
        help="Path to deployments.json (default: <repo-root>/deployments.json).",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=15,
        metavar="SECONDS",
        help="HTTP timeout for RPC calls (default: 15).",
    )
    args = parser.parse_args(argv)

    # Resolve manifest path
    if args.manifest:
        manifest_path = os.path.abspath(args.manifest)
    else:
        script_dir = os.path.dirname(os.path.abspath(__file__))
        root_dir = os.path.dirname(script_dir)
        manifest_path = os.path.join(root_dir, "deployments.json")

    print(f"Validating deployment manifest: {manifest_path}")

    # ── Load ────────────────────────────────────────────────────────────────
    if not os.path.exists(manifest_path):
        print(f"Error: Manifest file '{manifest_path}' does not exist.")
        return 1

    try:
        with open(manifest_path, "r", encoding="utf-8") as f:
            data = json.load(f)
    except json.JSONDecodeError as exc:
        print(f"Error: Manifest file is not valid JSON. {exc}")
        return 1

    # ── Schema validation ───────────────────────────────────────────────────
    print("\nRunning schema validation …")
    schema_errors = validate_schema(data)
    if schema_errors:
        print("\nSchema validation failed:")
        for err in schema_errors:
            print(f"  - {err}")
        return 1
    print("✓ Schema validation passed.")

    # ── On-chain verification ───────────────────────────────────────────────
    if args.verify_onchain:
        rpc_urls: dict[str, str] = {
            "testnet": args.rpc_testnet,
            "mainnet": args.rpc_mainnet,
        }
        # Filter to a single network if requested
        if args.network:
            rpc_urls = {args.network: rpc_urls[args.network]}

        print(f"\nRunning on-chain WASM hash verification …")
        onchain_errors = verify_onchain(data, rpc_urls)
        if onchain_errors:
            print("\nOn-chain verification failed:")
            for err in onchain_errors:
                print(f"  - {err}")
            return 1
        print("✓ On-chain WASM hash verification passed.")

    print("\n✓ Deployment manifest validation passed successfully!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
