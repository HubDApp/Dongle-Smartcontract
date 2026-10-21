#!/usr/bin/env python3
"""Validate CHANGELOG.md against the Keep a Changelog 1.1.0 conventions.

Checks performed:
  1. CHANGELOG.md exists at the repository root and is non-empty.
  2. It starts with a top level ``# Changelog`` heading.
  3. It references Keep a Changelog and Semantic Versioning.
  4. It contains an ``## [Unreleased]`` section, placed before every release.
  5. Every release heading matches ``## [X.Y.Z] - YYYY-MM-DD`` with a valid
     SemVer version and a valid ISO-8601 calendar date.
  6. Release versions are unique and listed newest first (descending SemVer),
     with dates that never move forward as you scroll down.
  7. Every ``###`` subsection uses one of the six Keep a Changelog categories
     and is not empty.
  8. Every version (including ``Unreleased``) has a matching link reference
     definition at the bottom of the file, and no link definition is orphaned.
  9. The newest released version matches the ``version`` field of the
     ``dongle-contract`` crate manifest.

Exit code 0 on success, 1 on any validation error.
"""

from __future__ import annotations

import datetime
import os
import re
import sys

ALLOWED_SECTIONS = (
    "Added",
    "Changed",
    "Deprecated",
    "Removed",
    "Fixed",
    "Security",
)

UNRELEASED_RE = re.compile(r"^##\s+\[Unreleased\]\s*$")
RELEASE_RE = re.compile(r"^##\s+\[(?P<version>[^\]]+)\]\s+-\s+(?P<date>\S+)\s*$")
ANY_H2_RE = re.compile(r"^##\s+(?P<title>.+?)\s*$")
H3_RE = re.compile(r"^###\s+(?P<title>.+?)\s*$")
LINK_DEF_RE = re.compile(r"^\[(?P<label>[^\]]+)\]:\s*(?P<url>\S+)\s*$")
SEMVER_RE = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-(?P<pre>[0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$"
)
CRATE_VERSION_RE = re.compile(r'^version\s*=\s*"([^"]+)"', re.MULTILINE)


def semver_key(version: str):
    """Sort key for SemVer strings; pre-releases order before their release."""
    match = SEMVER_RE.match(version)
    major, minor, patch = (int(part) for part in match.groups()[:3])
    pre = match.group("pre")
    # A version without a pre-release sorts after one with a pre-release.
    return (major, minor, patch, 1 if pre is None else 0, pre or "")


def crate_version(root_dir: str) -> str | None:
    manifest = os.path.join(root_dir, "dongle-smartcontract", "Cargo.toml")
    if not os.path.exists(manifest):
        return None
    with open(manifest, "r", encoding="utf-8") as handle:
        # Only look at the [package] table, which comes first in this manifest.
        content = handle.read().split("[lib]")[0]
    match = CRATE_VERSION_RE.search(content)
    return match.group(1) if match else None


def validate(path: str, root_dir: str) -> list[str]:
    errors: list[str] = []

    if not os.path.exists(path):
        return [
            f"{path} does not exist. Every release-bearing repository must "
            "track version history in a Keep a Changelog file."
        ]

    with open(path, "r", encoding="utf-8") as handle:
        lines = handle.read().splitlines()

    if not [line for line in lines if line.strip()]:
        return [f"{path} is empty."]

    # 2 + 3: header requirements.
    if lines[0].strip() != "# Changelog":
        errors.append("Line 1 must be the top-level heading '# Changelog'.")

    body = "\n".join(lines).lower()
    if "keepachangelog.com" not in body:
        errors.append("Missing a reference to the Keep a Changelog format.")
    if "semver.org" not in body:
        errors.append("Missing a reference to Semantic Versioning.")

    # Parse structure.
    releases: list[tuple[int, str, str]] = []  # (line_no, version, date)
    labels: list[str] = []
    link_defs: dict[str, str] = {}
    seen_unreleased = False
    current_heading: str | None = None
    current_section: str | None = None
    section_has_content = True
    in_code_fence = False

    for index, raw in enumerate(lines, start=1):
        line = raw.rstrip()

        if line.lstrip().startswith("```"):
            in_code_fence = not in_code_fence
            continue
        if in_code_fence:
            continue

        link_match = LINK_DEF_RE.match(line)
        if link_match:
            label = link_match.group("label")
            if label in link_defs:
                errors.append(f"Line {index}: duplicate link definition [{label}].")
            link_defs[label] = link_match.group("url")
            continue

        h2 = ANY_H2_RE.match(line)
        if h2:
            if current_section is not None and not section_has_content:
                errors.append(
                    f"Section '### {current_section}' under "
                    f"'{current_heading}' has no entries."
                )
            current_section = None
            section_has_content = True

            title = h2.group("title")
            if UNRELEASED_RE.match(line):
                if seen_unreleased:
                    errors.append(f"Line {index}: duplicate '## [Unreleased]' section.")
                if releases:
                    errors.append(
                        f"Line {index}: '## [Unreleased]' must appear before all "
                        "released versions."
                    )
                seen_unreleased = True
                labels.append("Unreleased")
                current_heading = "Unreleased"
                continue

            release = RELEASE_RE.match(line)
            if release:
                version = release.group("version")
                date = release.group("date")
                current_heading = line.strip()

                if not SEMVER_RE.match(version):
                    errors.append(
                        f"Line {index}: '{version}' is not a valid Semantic "
                        "Version (expected MAJOR.MINOR.PATCH)."
                    )
                try:
                    datetime.date.fromisoformat(date)
                except ValueError:
                    errors.append(
                        f"Line {index}: '{date}' is not a valid ISO-8601 date "
                        "(expected YYYY-MM-DD)."
                    )
                releases.append((index, version, date))
                labels.append(version)
                continue

            # Any other H2 is narrative prose (e.g. "How to add an entry"),
            # which is allowed only before the first version section.
            if seen_unreleased or releases:
                errors.append(
                    f"Line {index}: unexpected heading '## {title}'. Version "
                    "headings must be '## [Unreleased]' or "
                    "'## [X.Y.Z] - YYYY-MM-DD'."
                )
            current_heading = f"## {title}"
            continue

        h3 = H3_RE.match(line)
        if h3:
            if current_section is not None and not section_has_content:
                errors.append(
                    f"Section '### {current_section}' under "
                    f"'{current_heading}' has no entries."
                )
            title = h3.group("title")
            if seen_unreleased or releases:
                if title not in ALLOWED_SECTIONS:
                    errors.append(
                        f"Line {index}: '### {title}' is not a Keep a Changelog "
                        f"category. Use one of: {', '.join(ALLOWED_SECTIONS)}."
                    )
                current_section = title
                section_has_content = False
            continue

        if current_section is not None and line.strip():
            section_has_content = True

    if current_section is not None and not section_has_content:
        errors.append(
            f"Section '### {current_section}' under '{current_heading}' has no entries."
        )

    # 4: Unreleased must exist.
    if not seen_unreleased:
        errors.append("Missing an '## [Unreleased]' section.")

    # 5/6: releases present, unique and ordered newest first.
    if not releases:
        errors.append("No released versions found ('## [X.Y.Z] - YYYY-MM-DD').")

    seen_versions: dict[str, int] = {}
    for line_no, version, _date in releases:
        if version in seen_versions:
            errors.append(
                f"Line {line_no}: version {version} is declared more than once "
                f"(first seen on line {seen_versions[version]})."
            )
        else:
            seen_versions[version] = line_no

    valid = [
        (line_no, version, date)
        for line_no, version, date in releases
        if SEMVER_RE.match(version)
    ]
    for (prev_line, prev_v, prev_d), (line_no, version, date) in zip(valid, valid[1:]):
        if semver_key(version) >= semver_key(prev_v):
            errors.append(
                f"Line {line_no}: version {version} must be older than {prev_v} "
                f"declared on line {prev_line} (newest first)."
            )
        try:
            if datetime.date.fromisoformat(date) > datetime.date.fromisoformat(prev_d):
                errors.append(
                    f"Line {line_no}: date {date} of {version} is newer than "
                    f"{prev_d} of {prev_v} (newest first)."
                )
        except ValueError:
            pass  # Already reported above.

    # 8: link references.
    for label in labels:
        if label not in link_defs:
            errors.append(
                f"Missing link reference definition for [{label}] at the bottom "
                "of the file."
            )
    for label in link_defs:
        if label not in labels:
            errors.append(
                f"Link reference [{label}] does not match any version heading."
            )

    # 9: crate version agreement.
    latest_released = valid[0][1] if valid else None
    manifest_version = crate_version(root_dir)
    if latest_released and manifest_version and latest_released != manifest_version:
        errors.append(
            f"Latest released changelog version {latest_released} does not match "
            f"the dongle-contract crate version {manifest_version} in "
            "dongle-smartcontract/Cargo.toml."
        )

    return errors


def main() -> int:
    script_dir = os.path.dirname(os.path.abspath(__file__))
    root_dir = os.path.dirname(script_dir)
    changelog_path = os.path.join(root_dir, "CHANGELOG.md")

    print(f"Validating changelog: {changelog_path}")
    errors = validate(changelog_path, root_dir)

    if errors:
        print(f"\n{len(errors)} problem(s) found:\n")
        for error in errors:
            print(f"  - {error}")
        print(
            "\nSee https://keepachangelog.com/en/1.1.0/ and "
            "docs/CONTRIBUTING.md#6-changelog-entries"
        )
        return 1

    print("CHANGELOG.md is valid (Keep a Changelog 1.1.0 + SemVer).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
