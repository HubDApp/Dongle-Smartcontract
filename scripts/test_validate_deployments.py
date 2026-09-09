#!/usr/bin/env python3
"""Self-tests for scripts/validate_deployments.py.

Run with:
    python3 scripts/test_validate_deployments.py

No third-party dependencies required — everything uses the Python standard
library so the suite runs in any CI environment.

Coverage
--------
- Schema validation: valid manifest, missing fields, extra fields, bad
  contract_id / wasm_hash / git_commit / git_tag / deployer / timestamp formats.
- Normalization edge cases: uppercase wasm_hash, empty arrays.
- On-chain verification: fetch_onchain_wasm_hash and verify_onchain are tested
  with a mock HTTP server running on localhost so no real Stellar node is needed.
"""

from __future__ import annotations

import copy
import json
import os
import sys
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from typing import Any
from unittest.mock import MagicMock, patch

# Make sure we can import the module under test regardless of cwd.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from validate_deployments import (  # noqa: E402
    _strkey_decode,
    fetch_onchain_wasm_hash,
    validate_schema,
    verify_onchain,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

VALID_CONTRACT_ID = "CCWUXOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N73"
VALID_WASM_HASH = "a4b5c6d7e8f901a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f901a2b3c4d5e6f7"
VALID_GIT_COMMIT = "e069ec8"
VALID_DEPLOYER = "GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N"
VALID_TIMESTAMP = "2026-06-24T12:00:00Z"

VALID_ENTRY: dict[str, str] = {
    "contract_id": VALID_CONTRACT_ID,
    "wasm_hash": VALID_WASM_HASH,
    "git_commit": VALID_GIT_COMMIT,
    "deployer": VALID_DEPLOYER,
    "timestamp": VALID_TIMESTAMP,
}

VALID_MANIFEST: dict[str, Any] = {
    "$schema": "./deployments.schema.json",
    "testnet": [copy.deepcopy(VALID_ENTRY)],
    "mainnet": [],
}


def _manifest(**overrides: Any) -> dict[str, Any]:
    """Return a deep-copied valid manifest with optional field overrides in testnet[0]."""
    m = copy.deepcopy(VALID_MANIFEST)
    if overrides:
        for key, value in overrides.items():
            if value is _REMOVE:
                m["testnet"][0].pop(key, None)
            else:
                m["testnet"][0][key] = value
    return m


class _Remove:
    """Sentinel that signals a key should be deleted from the entry."""


_REMOVE = _Remove()


# ---------------------------------------------------------------------------
# Schema validation tests
# ---------------------------------------------------------------------------

class TestValidateSchema(unittest.TestCase):

    # ── Happy path ──────────────────────────────────────────────────────────

    def test_valid_manifest_no_errors(self):
        errors = validate_schema(VALID_MANIFEST)
        self.assertEqual(errors, [], f"Unexpected errors: {errors}")

    def test_valid_manifest_empty_arrays(self):
        errors = validate_schema({"testnet": [], "mainnet": []})
        self.assertEqual(errors, [])

    def test_valid_manifest_with_git_tag(self):
        m = _manifest(git_tag="v1.2.3")
        errors = validate_schema(m)
        self.assertEqual(errors, [])

    def test_valid_manifest_mainnet_entry(self):
        m = copy.deepcopy(VALID_MANIFEST)
        m["mainnet"].append(copy.deepcopy(VALID_ENTRY))
        errors = validate_schema(m)
        self.assertEqual(errors, [])

    def test_valid_manifest_without_schema_key(self):
        m = {"testnet": [], "mainnet": []}
        errors = validate_schema(m)
        self.assertEqual(errors, [])

    # ── Top-level structure ─────────────────────────────────────────────────

    def test_missing_testnet_key(self):
        errors = validate_schema({"mainnet": []})
        self.assertTrue(any("testnet" in e for e in errors))

    def test_missing_mainnet_key(self):
        errors = validate_schema({"testnet": []})
        self.assertTrue(any("mainnet" in e for e in errors))

    def test_extra_top_level_key(self):
        m = copy.deepcopy(VALID_MANIFEST)
        m["staging"] = []
        errors = validate_schema(m)
        self.assertTrue(any("staging" in e for e in errors))

    def test_root_not_object(self):
        errors = validate_schema([])  # type: ignore[arg-type]
        self.assertTrue(len(errors) > 0)

    def test_network_not_array(self):
        errors = validate_schema({"testnet": "oops", "mainnet": []})
        self.assertTrue(any("array" in e.lower() for e in errors))

    # ── Required fields ─────────────────────────────────────────────────────

    def test_missing_contract_id(self):
        errors = validate_schema(_manifest(contract_id=_REMOVE))
        self.assertTrue(any("contract_id" in e for e in errors))

    def test_missing_wasm_hash(self):
        errors = validate_schema(_manifest(wasm_hash=_REMOVE))
        self.assertTrue(any("wasm_hash" in e for e in errors))

    def test_missing_git_commit(self):
        errors = validate_schema(_manifest(git_commit=_REMOVE))
        self.assertTrue(any("git_commit" in e for e in errors))

    def test_missing_deployer(self):
        errors = validate_schema(_manifest(deployer=_REMOVE))
        self.assertTrue(any("deployer" in e for e in errors))

    def test_missing_timestamp(self):
        errors = validate_schema(_manifest(timestamp=_REMOVE))
        self.assertTrue(any("timestamp" in e for e in errors))

    def test_extra_field_rejected(self):
        errors = validate_schema(_manifest(unknown_field="value"))
        self.assertTrue(any("unexpected" in e.lower() for e in errors))

    # ── contract_id ─────────────────────────────────────────────────────────

    def test_contract_id_wrong_prefix(self):
        errors = validate_schema(_manifest(contract_id="G" + "A" * 55))
        self.assertTrue(any("contract_id" in e for e in errors))

    def test_contract_id_too_short(self):
        errors = validate_schema(_manifest(contract_id="CABC"))
        self.assertTrue(any("contract_id" in e for e in errors))

    def test_contract_id_too_long(self):
        errors = validate_schema(_manifest(contract_id="C" + "A" * 56))
        self.assertTrue(any("contract_id" in e for e in errors))

    def test_contract_id_invalid_chars(self):
        # Stellar base32 excludes 0,1,8,9
        errors = validate_schema(_manifest(contract_id="C" + "0" * 55))
        self.assertTrue(any("contract_id" in e for e in errors))

    # ── wasm_hash ────────────────────────────────────────────────────────────

    def test_wasm_hash_too_short(self):
        errors = validate_schema(_manifest(wasm_hash="deadbeef"))
        self.assertTrue(any("wasm_hash" in e for e in errors))

    def test_wasm_hash_too_long(self):
        errors = validate_schema(_manifest(wasm_hash="a" * 65))
        self.assertTrue(any("wasm_hash" in e for e in errors))

    def test_wasm_hash_non_hex(self):
        errors = validate_schema(_manifest(wasm_hash="g" * 64))
        self.assertTrue(any("wasm_hash" in e for e in errors))

    def test_wasm_hash_uppercase_accepted(self):
        # Schema pattern allows [a-fA-F0-9]
        errors = validate_schema(_manifest(wasm_hash=VALID_WASM_HASH.upper()))
        self.assertEqual(errors, [])

    # ── git_commit ───────────────────────────────────────────────────────────

    def test_git_commit_too_short(self):
        errors = validate_schema(_manifest(git_commit="abc"))
        self.assertTrue(any("git_commit" in e for e in errors))

    def test_git_commit_too_long(self):
        errors = validate_schema(_manifest(git_commit="a" * 41))
        self.assertTrue(any("git_commit" in e for e in errors))

    def test_git_commit_uppercase_rejected(self):
        errors = validate_schema(_manifest(git_commit="ABCDEFG"))
        self.assertTrue(any("git_commit" in e for e in errors))

    def test_git_commit_full_sha_accepted(self):
        errors = validate_schema(_manifest(git_commit="a" * 40))
        self.assertEqual(errors, [])

    # ── git_tag ──────────────────────────────────────────────────────────────

    def test_git_tag_valid_semver(self):
        errors = validate_schema(_manifest(git_tag="v1.2.3"))
        self.assertEqual(errors, [])

    def test_git_tag_with_hyphen(self):
        errors = validate_schema(_manifest(git_tag="release-1.0"))
        self.assertEqual(errors, [])

    def test_git_tag_with_space_rejected(self):
        errors = validate_schema(_manifest(git_tag="bad tag"))
        self.assertTrue(any("git_tag" in e for e in errors))

    # ── deployer ─────────────────────────────────────────────────────────────

    def test_deployer_starts_with_g(self):
        # 'G' prefix (regular account) should be accepted
        errors = validate_schema(_manifest(deployer=VALID_DEPLOYER))
        self.assertEqual(errors, [])

    def test_deployer_starts_with_c(self):
        # 'C' prefix (contract account) should also be accepted
        errors = validate_schema(_manifest(deployer=VALID_CONTRACT_ID))
        self.assertEqual(errors, [])

    def test_deployer_wrong_prefix(self):
        errors = validate_schema(_manifest(deployer="X" + "A" * 55))
        self.assertTrue(any("deployer" in e for e in errors))

    # ── timestamp ────────────────────────────────────────────────────────────

    def test_timestamp_z_suffix_accepted(self):
        errors = validate_schema(_manifest(timestamp="2026-01-01T00:00:00Z"))
        self.assertEqual(errors, [])

    def test_timestamp_offset_accepted(self):
        errors = validate_schema(_manifest(timestamp="2026-01-01T00:00:00+05:30"))
        self.assertEqual(errors, [])

    def test_timestamp_fractional_seconds_accepted(self):
        errors = validate_schema(_manifest(timestamp="2026-01-01T00:00:00.123Z"))
        self.assertEqual(errors, [])

    def test_timestamp_no_time_rejected(self):
        errors = validate_schema(_manifest(timestamp="2026-01-01"))
        self.assertTrue(any("timestamp" in e for e in errors))

    def test_timestamp_invalid_month(self):
        errors = validate_schema(_manifest(timestamp="2026-13-01T00:00:00Z"))
        self.assertTrue(any("timestamp" in e for e in errors))


# ---------------------------------------------------------------------------
# _strkey_decode tests
# ---------------------------------------------------------------------------

class TestStrKeyDecode(unittest.TestCase):

    def test_contract_id_decodes_to_32_bytes(self):
        result = _strkey_decode(VALID_CONTRACT_ID)
        self.assertEqual(len(result), 32)

    def test_account_id_decodes_to_32_bytes(self):
        result = _strkey_decode(VALID_DEPLOYER)
        self.assertEqual(len(result), 32)

    def test_invalid_strkey_raises(self):
        with self.assertRaises(Exception):
            _strkey_decode("NOTAVALIDSTRKEY")


# ---------------------------------------------------------------------------
# On-chain verification tests (mock HTTP server)
# ---------------------------------------------------------------------------

def _make_rpc_server(response_body: dict[str, Any]) -> tuple[HTTPServer, str]:
    """Start a local HTTP server that returns *response_body* for any POST request."""

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            length = int(self.headers.get("Content-Length", 0))
            self.rfile.read(length)
            body = json.dumps(response_body).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):  # silence request logs during tests
            pass

    server = HTTPServer(("127.0.0.1", 0), Handler)
    port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever)
    thread.daemon = True
    thread.start()
    return server, f"http://127.0.0.1:{port}"


class TestFetchOnchainWasmHash(unittest.TestCase):

    def _rpc_result_with_wasm_hash(self, wasm_hash: str) -> dict[str, Any]:
        """Build a minimal getLedgerEntries RPC response with a wasmHash convenience field."""
        return {
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "entries": [{"wasmHash": wasm_hash}],
                "latestLedger": 12345,
            },
        }

    def test_returns_wasm_hash_from_convenience_field(self):
        expected = VALID_WASM_HASH.lower()
        server, url = _make_rpc_server(self._rpc_result_with_wasm_hash(expected))
        try:
            result = fetch_onchain_wasm_hash(VALID_CONTRACT_ID, url)
            self.assertEqual(result, expected)
        finally:
            server.shutdown()

    def test_wasm_hash_normalised_to_lowercase(self):
        server, url = _make_rpc_server(
            self._rpc_result_with_wasm_hash(VALID_WASM_HASH.upper())
        )
        try:
            result = fetch_onchain_wasm_hash(VALID_CONTRACT_ID, url)
            self.assertEqual(result, VALID_WASM_HASH.lower())
        finally:
            server.shutdown()

    def test_contract_not_found_raises(self):
        server, url = _make_rpc_server({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"entries": [], "latestLedger": 1},
        })
        try:
            with self.assertRaises(RuntimeError) as ctx:
                fetch_onchain_wasm_hash(VALID_CONTRACT_ID, url)
            self.assertIn("not found", str(ctx.exception).lower())
        finally:
            server.shutdown()

    def test_rpc_error_raises(self):
        server, url = _make_rpc_server({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {"code": -32600, "message": "Invalid request"},
        })
        try:
            with self.assertRaises(RuntimeError) as ctx:
                fetch_onchain_wasm_hash(VALID_CONTRACT_ID, url)
            self.assertIn("rpc error", str(ctx.exception).lower())
        finally:
            server.shutdown()

    def test_network_error_raises(self):
        # Nothing is listening on this port
        with self.assertRaises(RuntimeError) as ctx:
            fetch_onchain_wasm_hash(VALID_CONTRACT_ID, "http://127.0.0.1:1")
        self.assertIn("network error", str(ctx.exception).lower())


class TestVerifyOnchain(unittest.TestCase):
    """Integration-style tests for verify_onchain using the mock server."""

    def _server_with_hash(self, wasm_hash: str) -> tuple[HTTPServer, str]:
        response = {
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "entries": [{"wasmHash": wasm_hash}],
                "latestLedger": 1,
            },
        }
        return _make_rpc_server(response)

    def test_matching_hash_produces_no_errors(self):
        server, url = self._server_with_hash(VALID_WASM_HASH)
        try:
            errors = verify_onchain(
                VALID_MANIFEST,
                {"testnet": url, "mainnet": url},
            )
            self.assertEqual(errors, [], f"Unexpected errors: {errors}")
        finally:
            server.shutdown()

    def test_mismatched_hash_produces_error(self):
        different_hash = "b" * 64
        server, url = self._server_with_hash(different_hash)
        try:
            errors = verify_onchain(
                VALID_MANIFEST,
                {"testnet": url, "mainnet": url},
            )
            self.assertTrue(
                any("mismatch" in e.lower() for e in errors),
                f"Expected mismatch error, got: {errors}",
            )
        finally:
            server.shutdown()

    def test_mismatch_error_includes_both_hashes(self):
        different_hash = "b" * 64
        server, url = self._server_with_hash(different_hash)
        try:
            errors = verify_onchain(
                VALID_MANIFEST,
                {"testnet": url, "mainnet": url},
            )
            self.assertTrue(len(errors) == 1)
            self.assertIn(different_hash, errors[0])
            self.assertIn(VALID_WASM_HASH.lower(), errors[0])
        finally:
            server.shutdown()

    def test_missing_rpc_url_skips_network(self):
        # No URL provided for testnet → should skip silently, no errors
        errors = verify_onchain(VALID_MANIFEST, {})
        self.assertEqual(errors, [])

    def test_network_error_produces_error(self):
        errors = verify_onchain(
            VALID_MANIFEST,
            {"testnet": "http://127.0.0.1:1"},  # nothing listening
        )
        self.assertTrue(
            any("failed to fetch" in e.lower() for e in errors),
            f"Expected fetch-failure error, got: {errors}",
        )

    def test_empty_entries_no_errors(self):
        errors = verify_onchain(
            {"testnet": [], "mainnet": []},
            {"testnet": "http://127.0.0.1:1", "mainnet": "http://127.0.0.1:1"},
        )
        self.assertEqual(errors, [])

    def test_only_requested_network_checked(self):
        """Providing only mainnet RPC should skip testnet entries."""
        different_hash = "b" * 64
        server, url = self._server_with_hash(different_hash)
        try:
            # Only pass mainnet URL; testnet has the entry but no URL → skipped
            errors = verify_onchain(VALID_MANIFEST, {"mainnet": url})
            self.assertEqual(errors, [], f"Unexpected errors: {errors}")
        finally:
            server.shutdown()

    def test_uppercase_manifest_hash_normalised_before_compare(self):
        """wasm_hash stored in uppercase in the manifest should still match."""
        m = copy.deepcopy(VALID_MANIFEST)
        m["testnet"][0]["wasm_hash"] = VALID_WASM_HASH.upper()
        # Server returns lowercase hash
        server, url = self._server_with_hash(VALID_WASM_HASH.lower())
        try:
            errors = verify_onchain(m, {"testnet": url, "mainnet": url})
            self.assertEqual(errors, [], f"Unexpected errors: {errors}")
        finally:
            server.shutdown()


# ---------------------------------------------------------------------------
# CLI integration test (no network)
# ---------------------------------------------------------------------------

class TestCLI(unittest.TestCase):
    """Exercise the main() entry point through its argument parser."""

    def _write_manifest(self, path: str, data: dict) -> None:
        with open(path, "w", encoding="utf-8") as f:
            json.dump(data, f)

    def test_valid_manifest_exits_zero(self):
        import tempfile
        with tempfile.NamedTemporaryFile(suffix=".json", mode="w", delete=False) as f:
            json.dump(VALID_MANIFEST, f)
            path = f.name
        try:
            from validate_deployments import main
            rc = main(["--manifest", path])
            self.assertEqual(rc, 0)
        finally:
            os.unlink(path)

    def test_invalid_manifest_exits_nonzero(self):
        import tempfile
        bad = copy.deepcopy(VALID_MANIFEST)
        bad["testnet"][0]["wasm_hash"] = "tooshort"
        with tempfile.NamedTemporaryFile(suffix=".json", mode="w", delete=False) as f:
            json.dump(bad, f)
            path = f.name
        try:
            from validate_deployments import main
            rc = main(["--manifest", path])
            self.assertNotEqual(rc, 0)
        finally:
            os.unlink(path)

    def test_missing_manifest_exits_nonzero(self):
        from validate_deployments import main
        rc = main(["--manifest", "/nonexistent/path/deployments.json"])
        self.assertNotEqual(rc, 0)


# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    result = unittest.main(exit=False, verbosity=2)
    sys.exit(0 if result.result.wasSuccessful() else 1)
