#!/usr/bin/env python3
"""Self-tests for scripts/validate_changelog.py.

Run with:  python3 scripts/test_validate_changelog.py

No third-party dependencies required so it can run anywhere CI runs Python.
"""

from __future__ import annotations

import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from validate_changelog import validate  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

HEADER = """# Changelog

All notable changes are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
"""

GOOD = (
    HEADER
    + """
## [Unreleased]

### Added

- Something new.

## [0.2.0] - 2026-02-01

### Changed

- Something changed.

## [0.1.0] - 2025-09-25

### Added

- Initial release.

[Unreleased]: https://example.com/compare/v0.2.0...HEAD
[0.2.0]: https://example.com/compare/v0.1.0...v0.2.0
[0.1.0]: https://example.com/releases/tag/v0.1.0
"""
)

FAILURES = 0


def check(name: str, content: str | None, expect_ok: bool, needle: str = "") -> None:
    global FAILURES
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "CHANGELOG.md")
        if content is not None:
            with open(path, "w", encoding="utf-8") as handle:
                handle.write(content)
        errors = validate(path, tmp)
        ok = not errors
        passed = ok == expect_ok and (not needle or any(needle in e for e in errors))
        status = "PASS" if passed else "FAIL"
        if not passed:
            FAILURES += 1
        print(f"[{status}] {name}")
        if not passed:
            print(f"         expected_ok={expect_ok} errors={errors}")


def main() -> int:
    check("valid changelog accepted", GOOD, True)
    check("missing file rejected", None, False, "does not exist")
    check("empty file rejected", "", False, "empty")
    check("wrong title rejected", GOOD.replace("# Changelog", "# Notes", 1), False, "top-level heading")
    check(
        "missing Keep a Changelog reference rejected",
        GOOD.replace("https://keepachangelog.com/en/1.1.0/", "https://example.com"),
        False,
        "Keep a Changelog format",
    )
    check(
        "missing Unreleased rejected",
        GOOD.replace("## [Unreleased]\n\n### Added\n\n- Something new.\n", "").replace(
            "[Unreleased]: https://example.com/compare/v0.2.0...HEAD\n", ""
        ),
        False,
        "Unreleased",
    )
    check(
        "non-semver version rejected",
        GOOD.replace("## [0.2.0] - 2026-02-01", "## [2026-02-01] - 2026-02-01"),
        False,
        "Semantic Version",
    )
    check(
        "bad date rejected",
        GOOD.replace("## [0.2.0] - 2026-02-01", "## [0.2.0] - 2026-13-01"),
        False,
        "ISO-8601",
    )
    check(
        "unknown section rejected",
        GOOD.replace("### Changed", "### Improvements"),
        False,
        "Keep a Changelog category",
    )
    check(
        "empty section rejected",
        GOOD.replace("### Changed\n\n- Something changed.\n", "### Changed\n"),
        False,
        "no entries",
    )
    check(
        "out-of-order versions rejected",
        GOOD.replace("## [0.2.0] - 2026-02-01", "## [0.0.9] - 2026-02-01").replace(
            "[0.2.0]:", "[0.0.9]:"
        ),
        False,
        "must be older than",
    )
    check(
        "missing link reference rejected",
        GOOD.replace("[0.1.0]: https://example.com/releases/tag/v0.1.0\n", ""),
        False,
        "Missing link reference",
    )
    check(
        "orphan link reference rejected",
        GOOD + "[9.9.9]: https://example.com/compare/v9.9.8...v9.9.9\n",
        False,
        "does not match any version heading",
    )
    check(
        "duplicate version rejected",
        GOOD.replace("## [0.1.0] - 2025-09-25", "## [0.2.0] - 2025-09-25"),
        False,
        "more than once",
    )

    # The real repository changelog must also validate.
    real_errors = validate(os.path.join(ROOT, "CHANGELOG.md"), ROOT)
    if real_errors:
        print(f"[FAIL] repository CHANGELOG.md valid -> {real_errors}")
        globals()["FAILURES"] += 1
    else:
        print("[PASS] repository CHANGELOG.md valid")

    if FAILURES:
        print(f"\n{FAILURES} test(s) failed.")
        return 1
    print("\nAll changelog validator tests passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
