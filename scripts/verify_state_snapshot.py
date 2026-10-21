#!/usr/bin/env python3
"""Validate a contract state snapshot export against its on-chain record (#933).

The snapshot file is the JSON printed by
`stellar contract invoke ... -- export_contract_state_snapshot`. The record is
the JSON printed by `get_state_snapshot`. See docs/STATE_SNAPSHOTS.md.

Checks:
  * the file's SHA-256 matches the record's `export_sha256`
  * `state_hash`, scope, format version and project range match the record
  * the snapshot is internally consistent: lists outside the scope are empty,
    project IDs are unique and inside the range, and every review and
    verification references a project in the range

Exit status is 0 when every check passes and 1 otherwise.
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

SUPPORTED_FORMAT_VERSIONS = {1}
SCOPE_LISTS = {
    "Full": {"projects", "reviews", "verifications", "config"},
    "Projects": {"projects"},
    "Reviews": {"reviews"},
    "Verifications": {"verifications"},
    "Configs": {"config"},
}


def load_json(path):
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def normalize_hex(value):
    if isinstance(value, list):
        return bytes(value).hex()
    return str(value).lower().removeprefix("0x")


def scope_name(value):
    # The CLI renders unit enum variants as a string, or as a one-key object
    # in some versions.
    if isinstance(value, dict) and len(value) == 1:
        return next(iter(value))
    if isinstance(value, list) and value:
        return str(value[0])
    return str(value)


def check_structure(snapshot, errors):
    version = snapshot.get("format_version")
    if version not in SUPPORTED_FORMAT_VERSIONS:
        errors.append(f"unsupported format_version {version!r}")

    scope = scope_name(snapshot.get("scope"))
    if scope not in SCOPE_LISTS:
        errors.append(f"unknown scope {scope!r}")
        return
    allowed = SCOPE_LISTS[scope]

    for field in ("projects", "reviews", "verifications"):
        items = snapshot.get(field) or []
        if field not in allowed and items:
            errors.append(f"scope {scope} must not contain {field}")
    config = snapshot.get("config")
    if ("config" in allowed) != (config is not None):
        errors.append(f"scope {scope} config presence is wrong")

    start = int(snapshot.get("start_project_id", 0))
    end = int(snapshot.get("end_project_id", 0))

    seen = set()
    for project in snapshot.get("projects") or []:
        pid = int(project["id"])
        if pid in seen:
            errors.append(f"project {pid} appears twice")
        seen.add(pid)
        if not start <= pid <= end:
            errors.append(f"project {pid} outside range {start}..{end}")

    for field in ("reviews", "verifications"):
        for item in snapshot.get(field) or []:
            pid = int(item["project_id"])
            if not start <= pid <= end:
                errors.append(f"{field[:-1]} for project {pid} outside range {start}..{end}")
            elif scope == "Full" and pid not in seen:
                errors.append(f"{field[:-1]} references project {pid} missing from snapshot")


def check_against_record(raw, snapshot, record, errors):
    file_hash = hashlib.sha256(raw).hexdigest()
    if file_hash != normalize_hex(record["export_sha256"]):
        errors.append(f"file SHA-256 {file_hash} does not match record export_sha256")

    if normalize_hex(snapshot.get("state_hash")) != normalize_hex(record["state_hash"]):
        errors.append("state_hash does not match record")

    for field in ("format_version", "start_project_id", "end_project_id"):
        if int(snapshot.get(field, -1)) != int(record[field]):
            errors.append(f"{field} {snapshot.get(field)!r} does not match record {record[field]!r}")
    if scope_name(snapshot.get("scope")) != scope_name(record["scope"]):
        errors.append("scope does not match record")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("snapshot", type=Path, help="snapshot JSON file")
    parser.add_argument("--record", type=Path, help="get_state_snapshot JSON output")
    parser.add_argument(
        "--print-sha256",
        action="store_true",
        help="print the file's SHA-256 (the export_sha256 to record) and exit",
    )
    args = parser.parse_args(argv)

    raw = args.snapshot.read_bytes()
    if args.print_sha256:
        print(hashlib.sha256(raw).hexdigest())
        return 0

    snapshot = json.loads(raw)
    errors = []
    check_structure(snapshot, errors)
    if args.record:
        check_against_record(raw, snapshot, load_json(args.record), errors)

    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print(
        f"OK: scope {scope_name(snapshot.get('scope'))}, projects "
        f"{snapshot.get('start_project_id')}..{snapshot.get('end_project_id')}, "
        f"{len(snapshot.get('projects') or [])} projects, "
        f"{len(snapshot.get('reviews') or [])} reviews, "
        f"{len(snapshot.get('verifications') or [])} verifications"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
