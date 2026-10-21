#!/usr/bin/env python3
"""Integration test for .pre-commit-config.yaml (#872).

1. Verifies the config declares the required Rust hooks (fmt, clippy, doc).
2. Executes every hook via `pre-commit run <id> --all-files` and fails if any
   hook does not run successfully.

Usage:
    python3 scripts/test_pre_commit_hooks.py            # check config + run hooks
    python3 scripts/test_pre_commit_hooks.py --no-exec  # config check only
"""

import shutil
import subprocess
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent.parent
CONFIG = ROOT / ".pre-commit-config.yaml"
REQUIRED_HOOKS = {
    "cargo-fmt": "cargo fmt",
    "cargo-clippy": "cargo clippy",
    "cargo-doc": "cargo doc",
}


def load_hooks():
    config = yaml.safe_load(CONFIG.read_text())
    hooks = {}
    for repo in config.get("repos", []):
        for hook in repo.get("hooks", []):
            hooks[hook["id"]] = hook
    return hooks


def check_config(hooks):
    errors = []
    for hook_id, command in REQUIRED_HOOKS.items():
        hook = hooks.get(hook_id)
        if hook is None:
            errors.append(f"missing hook '{hook_id}'")
        elif command not in hook.get("entry", ""):
            errors.append(f"hook '{hook_id}' entry does not run '{command}'")
    return errors


def run_hooks(hooks):
    if shutil.which("pre-commit") is None:
        return ["pre-commit is not installed (pip install pre-commit)"]
    errors = []
    for hook_id in hooks:
        print(f"running hook: {hook_id}")
        result = subprocess.run(
            ["pre-commit", "run", hook_id, "--all-files"], cwd=ROOT
        )
        if result.returncode != 0:
            errors.append(f"hook '{hook_id}' failed (exit {result.returncode})")
    return errors


def main():
    hooks = load_hooks()
    errors = check_config(hooks)
    if not errors and "--no-exec" not in sys.argv:
        errors = run_hooks(hooks)
    for error in errors:
        print(f"FAIL: {error}", file=sys.stderr)
    if errors:
        return 1
    print(f"OK: {len(hooks)} pre-commit hooks verified")
    return 0


if __name__ == "__main__":
    sys.exit(main())
