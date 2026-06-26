#!/usr/bin/env python3
import json
import os
import re
import sys

def main():
    # Find the repository root
    script_dir = os.path.dirname(os.path.abspath(__file__))
    root_dir = os.path.dirname(script_dir)
    manifest_path = os.path.join(root_dir, "deployments.json")

    print(f"Validating deployment manifest: {manifest_path}")

    if not os.path.exists(manifest_path):
        print(f"Error: Manifest file '{manifest_path}' does not exist.")
        sys.exit(1)

    try:
        with open(manifest_path, "r", encoding="utf-8") as f:
            data = json.load(f)
    except json.JSONDecodeError as e:
        print(f"Error: Manifest file is not valid JSON. {e}")
        sys.exit(1)

    # Validate structure
    if not isinstance(data, dict):
        print("Error: Root of manifest must be a JSON object.")
        sys.exit(1)

    # Check top-level keys
    allowed_keys = {"$schema", "testnet", "mainnet"}
    required_keys = {"testnet", "mainnet"}
    
    actual_keys = set(data.keys())
    
    missing_keys = required_keys - actual_keys
    if missing_keys:
        print(f"Error: Missing required top-level keys: {missing_keys}")
        sys.exit(1)

    extra_keys = actual_keys - allowed_keys
    if extra_keys:
        print(f"Error: Unexpected top-level keys: {extra_keys}")
        sys.exit(1)

    # Regex definitions matching the JSON schema
    contract_id_pattern = re.compile(r"^C[A-Z2-7]{55}$")
    wasm_hash_pattern = re.compile(r"^[a-fA-F0-9]{64}$")
    deployer_pattern = re.compile(r"^[GC][A-Z2-7]{55}$")
    # ISO 8601 format regex
    timestamp_pattern = re.compile(
        r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})$"
    )

    errors = []

    for network in ["testnet", "mainnet"]:
        entries = data[network]
        if not isinstance(entries, list):
            errors.append(f"Network '{network}' must be an array of deployment entries.")
            continue

        for idx, entry in enumerate(entries):
            loc = f"{network}[{idx}]"
            if not isinstance(entry, dict):
                errors.append(f"Entry {loc} must be a JSON object.")
                continue

            # Check for required properties
            entry_keys = set(entry.keys())
            req_fields = {"contract_id", "wasm_hash", "deployer", "timestamp"}
            
            missing_fields = req_fields - entry_keys
            if missing_fields:
                errors.append(f"Entry {loc} is missing required fields: {missing_fields}")
                
            extra_fields = entry_keys - req_fields
            if extra_fields:
                errors.append(f"Entry {loc} has unexpected fields: {extra_fields}")

            # If there are missing fields, skip format validation for those fields
            if "contract_id" in entry:
                cid = entry["contract_id"]
                if not isinstance(cid, str) or not contract_id_pattern.match(cid):
                    errors.append(
                        f"Entry {loc}.contract_id ('{cid}') is invalid. "
                        f"Must be a 56-character Stellar Contract ID starting with 'C'."
                    )

            if "wasm_hash" in entry:
                whash = entry["wasm_hash"]
                if not isinstance(whash, str) or not wasm_hash_pattern.match(whash):
                    errors.append(
                        f"Entry {loc}.wasm_hash ('{whash}') is invalid. "
                        f"Must be a 64-character hex string."
                    )

            if "deployer" in entry:
                deployer = entry["deployer"]
                if not isinstance(deployer, str) or not deployer_pattern.match(deployer):
                    errors.append(
                        f"Entry {loc}.deployer ('{deployer}') is invalid. "
                        f"Must be a 56-character Stellar account or contract ID starting with 'G' or 'C'."
                    )

            if "timestamp" in entry:
                ts = entry["timestamp"]
                if not isinstance(ts, str) or not timestamp_pattern.match(ts):
                    errors.append(
                        f"Entry {loc}.timestamp ('{ts}') is invalid. "
                        f"Must be a valid ISO 8601 / RFC 3339 datetime string (e.g. '2026-06-24T12:00:00Z')."
                    )

    if errors:
        print("\nValidation failed with the following errors:")
        for err in errors:
            print(f"  - {err}")
        sys.exit(1)

    print("\n✓ Deployment manifest validation passed successfully!")
    sys.exit(0)

if __name__ == "__main__":
    main()
