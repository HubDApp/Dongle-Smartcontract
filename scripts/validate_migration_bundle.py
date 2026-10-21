#!/usr/bin/env python3
"""Check migration bundles exported by `export_for_migration` (#935).

Pass every page of one export, in any order. The script checks each page's
metadata against its contents, that the pages share one source contract,
target version and format, and that together they cover project IDs
1..project_count with no gaps or overlaps.

It cannot recompute `content_hash` (that needs the XDR encoding); call the
contract's `validate_migration_bundle` for that. See docs/CONTRACT_MIGRATION.md.

Exit status is 0 when every check passes and 1 otherwise.
"""

import argparse
import json
import re
import sys
from pathlib import Path

SUPPORTED_FORMATS = {1}
VERSION_RE = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")


def parse_version(value):
    match = VERSION_RE.match(str(value))
    return tuple(int(part) for part in match.groups()) if match else None


def check_versions(source, target, errors, label):
    src, tgt = parse_version(source), parse_version(target)
    if src is None or tgt is None:
        errors.append(f"{label}: invalid version {source!r} -> {target!r}")
    elif tgt < src:
        errors.append(f"{label}: downgrade {source} -> {target} is not supported")
    elif tgt[0] > src[0] + 1:
        errors.append(f"{label}: {source} -> {target} jumps more than one major version")


def check_page(label, bundle, target, errors):
    meta = bundle["metadata"]
    if meta["format_version"] not in SUPPORTED_FORMATS:
        errors.append(f"{label}: unsupported format_version {meta['format_version']}")
    if target and meta["target_version"] != target:
        errors.append(f"{label}: prepared for {meta['target_version']}, not {target}")
    check_versions(meta["source_contract_version"], meta["target_version"], errors, label)

    for field, count in (
        ("projects", "project_records"),
        ("reviews", "review_records"),
        ("verifications", "verification_records"),
    ):
        if len(bundle.get(field) or []) != int(meta[count]):
            errors.append(f"{label}: {field} has {len(bundle.get(field) or [])} records, metadata says {meta[count]}")

    start, end = int(meta["start_project_id"]), int(meta["end_project_id"])
    ids = [int(p["id"]) for p in bundle.get("projects") or []]
    if len(ids) != len(set(ids)):
        errors.append(f"{label}: duplicate project IDs")
    for pid in ids + [int(r["project_id"]) for r in (bundle.get("reviews") or []) + (bundle.get("verifications") or [])]:
        if not start <= pid <= end:
            errors.append(f"{label}: record for project {pid} outside {start}..{end}")
            break


def check_coverage(pages, errors):
    first = pages[0][1]["metadata"]
    for label, bundle in pages[1:]:
        meta = bundle["metadata"]
        for field in ("source_contract", "target_version", "format_version", "project_count"):
            if meta[field] != first[field]:
                errors.append(f"{label}: {field} differs from other pages")

    ranges = sorted(
        (int(b["metadata"]["start_project_id"]), int(b["metadata"]["end_project_id"]), label)
        for label, b in pages
    )
    expected = 1
    for start, end, label in ranges:
        if end < start:
            continue  # empty page past the end
        if start != expected:
            errors.append(f"{label}: expected page to start at {expected}, got {start}")
        expected = end + 1
    total = int(first["project_count"])
    if expected - 1 < total:
        errors.append(f"pages cover projects up to {expected - 1} of {total}")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("bundles", nargs="+", type=Path, help="bundle JSON files, one per page")
    parser.add_argument("--target-version", help="version the bundles must be prepared for")
    args = parser.parse_args(argv)

    errors = []
    pages = []
    for path in args.bundles:
        with path.open(encoding="utf-8") as handle:
            bundle = json.load(handle)
        pages.append((path.name, bundle))
        check_page(path.name, bundle, args.target_version, errors)
    check_coverage(pages, errors)

    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1
    meta = pages[0][1]["metadata"]
    print(
        f"OK: {len(pages)} page(s), {meta['source_contract_version']} -> {meta['target_version']}, "
        f"format {meta['format_version']}, {meta['project_count']} projects"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
