#!/usr/bin/env python3
"""Validate registry exports and prepare bulk register_project payloads."""

import argparse
import csv
import json
import re
import sys
from datetime import datetime
from pathlib import Path
from urllib.parse import urlparse

OWNER_RE = re.compile(r"^G[A-Z2-7]{55}$")
SLUG_RE = re.compile(r"^[a-z0-9-]{1,64}$")
TAG_RE = re.compile(r"^[A-Za-z0-9_-]{1,32}$")
REQUIRED = ("sourceId", "owner", "name", "slug", "description", "category")
OPTIONAL = (
    "website", "repository", "license", "logoCid", "metadataCid",
    "tags", "socialLinks", "launchTimestamp", "bountyUrl", "maintainers",
)


def parse_json(path):
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def parse_pairs(value):
    if not value:
        return None
    result = {}
    for pair in value.split("|"):
        key, separator, item = pair.partition("=")
        if not separator or not key or not item:
            raise ValueError("socialLinks must use key=url pairs separated by '|'")
        result[key] = item
    return result


def parse_csv(path):
    with path.open(newline="", encoding="utf-8") as handle:
        rows = []
        for row in csv.DictReader(handle):
            row["tags"] = row.get("tags") or None
            if row["tags"]:
                row["tags"] = row["tags"].split("|")
            row["socialLinks"] = parse_pairs(row.get("socialLinks"))
            for field in ("launchTimestamp",):
                if row.get(field) == "":
                    row[field] = None
                elif row.get(field) is not None:
                    row[field] = int(row[field])
            for field in OPTIONAL:
                if row.get(field) == "":
                    row[field] = None
            rows.append(row)
    return {
        "schemaVersion": "1.0.0",
        "source": {"registry": "csv", "exportedAt": datetime.now().isoformat()},
        "projects": rows,
    }


def parse_github(path):
    raw = parse_json(path)
    items = raw if isinstance(raw, list) else raw.get("items", [])
    projects = []
    for item in items:
        projects.append({
            "sourceId": str(item.get("id", "")),
            "owner": item.get("stellar_owner"),
            "name": item.get("name"),
            "slug": item.get("dongle_slug"),
            "description": item.get("description") or item.get("name"),
            "category": item.get("dongle_category"),
            "website": item.get("homepage") or None,
            "repository": item.get("html_url") or None,
            "license": (item.get("license") or {}).get("spdx_id"),
            "tags": item.get("topics") or None,
            "socialLinks": None,
            "logoCid": None,
            "metadataCid": None,
            "launchTimestamp": None,
            "bountyUrl": None,
            "maintainers": [item["owner"]["login"]] if item.get("owner") else None,
        })
    return {
        "schemaVersion": "1.0.0",
        "source": {"registry": "github", "exportedAt": datetime.now().isoformat()},
        "projects": projects,
    }


def validate_url(value, field, errors):
    if value is not None:
        parsed = urlparse(value)
        if parsed.scheme not in ("http", "https") or not parsed.netloc:
            errors.append(f"{field} must be an http(s) URL")


def normalize_and_validate(document):
    errors = []
    if document.get("schemaVersion") != "1.0.0":
        errors.append("schemaVersion must be 1.0.0")
    source = document.get("source")
    if not isinstance(source, dict) or not isinstance(source.get("registry"), str) or not source["registry"].strip():
        errors.append("source.registry is required")
    if isinstance(source, dict):
        exported_at = source.get("exportedAt")
        if not isinstance(exported_at, str):
            errors.append("source.exportedAt is required")
        else:
            try:
                datetime.fromisoformat(exported_at.replace("Z", "+00:00"))
            except ValueError:
                errors.append("source.exportedAt must be an ISO-8601 timestamp")
    if not isinstance(document.get("projects"), list) or not document["projects"]:
        errors.append("projects must be a non-empty array")
        return [], errors

    source_ids = set()
    slugs = set()
    normalized = []
    for index, project in enumerate(document["projects"], start=1):
        prefix = f"projects[{index}]"
        if not isinstance(project, dict):
            errors.append(f"{prefix} must be an object")
            continue
        allowed = set(REQUIRED + OPTIONAL)
        for field in project:
            if field not in allowed:
                errors.append(f"{prefix}.{field} is not supported")
        for field in REQUIRED:
            if not isinstance(project.get(field), str) or not project[field].strip():
                errors.append(f"{prefix}.{field} is required")
        source_id = project.get("sourceId")
        slug = project.get("slug")
        if source_id in source_ids:
            errors.append(f"{prefix}.sourceId is duplicated")
        source_ids.add(source_id)
        if slug and not SLUG_RE.fullmatch(slug):
            errors.append(f"{prefix}.slug must contain only lowercase letters, digits, and hyphens")
        if slug in slugs:
            errors.append(f"{prefix}.slug is duplicated")
        slugs.add(slug)
        owner = project.get("owner")
        if owner and not OWNER_RE.fullmatch(owner):
            errors.append(f"{prefix}.owner must be a valid Stellar account address")
        for field, limit in (("name", 128), ("description", 4096), ("category", 64)):
            value = project.get(field)
            if isinstance(value, str) and len(value) > limit:
                errors.append(f"{prefix}.{field} exceeds {limit} characters")
        for field in ("website", "repository", "bountyUrl"):
            validate_url(project.get(field), f"{prefix}.{field}", errors)
        tags = project.get("tags")
        if tags is not None:
            if not isinstance(tags, list) or len(tags) > 10:
                errors.append(f"{prefix}.tags must contain at most 10 items")
            else:
                for tag in tags:
                    if not isinstance(tag, str) or not TAG_RE.fullmatch(tag):
                        errors.append(f"{prefix}.tags contains an invalid tag")
        socials = project.get("socialLinks")
        if socials is not None:
            if not isinstance(socials, dict):
                errors.append(f"{prefix}.socialLinks must be an object")
            else:
                for key, value in socials.items():
                    validate_url(value, f"{prefix}.socialLinks.{key}", errors)
        timestamp = project.get("launchTimestamp")
        if timestamp is not None and (not isinstance(timestamp, int) or timestamp < 0):
            errors.append(f"{prefix}.launchTimestamp must be a non-negative integer")
        normalized.append({
            "sourceId": source_id,
            "params": {
                "owner": owner,
                "name": project.get("name"),
                "slug": slug,
                "description": project.get("description"),
                "category": project.get("category"),
                "website": project.get("website"),
                "license": project.get("license"),
                "logo_cid": project.get("logoCid"),
                "metadata_cid": project.get("metadataCid"),
                "tags": project.get("tags"),
                "social_links": project.get("socialLinks"),
                "launch_timestamp": timestamp,
                "bounty_url": project.get("bountyUrl"),
                "repository_url": project.get("repository"),
            },
        })
    return normalized, errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("--format", choices=("canonical", "csv", "github"), default="canonical")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--start", type=int, default=0)
    parser.add_argument("--limit", type=int, default=100)
    args = parser.parse_args()
    if args.start < 0 or args.limit <= 0:
        parser.error("--start must be non-negative and --limit must be positive")

    try:
        document = parse_csv(args.input) if args.format == "csv" else parse_github(args.input) if args.format == "github" else parse_json(args.input)
        prepared, errors = normalize_and_validate(document)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"migration input error: {error}", file=sys.stderr)
        return 2
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    batch = prepared[args.start:args.start + args.limit]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(batch, indent=2) + "\n", encoding="utf-8")
    print(f"validated {len(prepared)} projects; prepared {len(batch)} payloads in {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
