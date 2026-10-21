#!/usr/bin/env python3
"""CI: compare fresh benchmark results against benchmarks/baseline.json.

Usage: check_benchmarks.py <results.json>
results.json format: {"samples": [{"name": str, "exec_units": int}, ...]}
Exits 1 and prints regressions exceeding the 10% (1000 bps) threshold.
"""
import json
import sys
from pathlib import Path

BASELINE = Path(__file__).parent.parent / "benchmarks" / "baseline.json"
THRESHOLD_BPS = 1000


def main() -> int:
    baseline = json.loads(BASELINE.read_text())["baselines"]
    base = {b["name"]: b["exec_units"] for b in baseline}
    results = json.loads(Path(sys.argv[1]).read_text())["samples"]
    regressions = []
    for s in results:
        b = base.get(s["name"], 0)
        if b and s["exec_units"] > b:
            bps = (s["exec_units"] - b) * 10_000 // b
            if bps > THRESHOLD_BPS:
                regressions.append((s["name"], b, s["exec_units"], bps / 100))
    if regressions:
        for name, b, cur, pct in regressions:
            print(f"REGRESSION: {name} baseline={b} current={cur} (+{pct:.1f}%)")
        return 1
    print("No benchmark regressions over 10%.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
