#!/usr/bin/env python3
"""Benchmark regression checker for gitprint.

Modes
-----
Default (no flags)
  Compare target/criterion/**/current/estimates.json against the stored
  baseline in bench-baseline/**/main/estimates.json.  Exits 1 if any
  benchmark regressed more than REGRESSION_THRESHOLD percent (default 20).

--save
  Promote target/criterion/**/current/ results to bench-baseline/**/main/
  without running any comparison.  Call this after a passing check to lock
  in the current results as the new baseline.

The REGRESSION_THRESHOLD environment variable overrides the default (20%).

Usage
-----
  # Check for regressions (fails fast):
  python3 scripts/check_benchmarks.py

  # Save current results as the new baseline (after a passing release):
  python3 scripts/check_benchmarks.py --save
"""
import glob
import json
import os
import shutil
import sys

CRITERION_DIR = "target/criterion"
BASELINE_DIR = "bench-baseline"


def _load_mean(path: str) -> float:
    with open(path) as f:
        return json.load(f)["mean"]["point_estimate"]


def save() -> None:
    """Copy target/criterion/**/current/estimates.json → bench-baseline/**/main/…"""
    saved = 0
    for src in glob.glob(f"{CRITERION_DIR}/**/current/estimates.json", recursive=True):
        rel = os.path.relpath(src, CRITERION_DIR)
        dest = os.path.join(BASELINE_DIR, rel.replace("/current/", "/main/"))
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.copy(src, dest)
        saved += 1
    print(f"Baseline updated: {saved} result(s) saved to {BASELINE_DIR}/.")


def check(threshold_pct: float = 20.0) -> bool:
    """Return True if all benchmarks are within threshold, False otherwise."""
    if not os.path.isdir(BASELINE_DIR):
        print(f"No baseline found in {BASELINE_DIR}/.")
        print("Run 'make bench-baseline' once to establish one, then re-run.")
        print("Skipping regression check for this run.")
        return True  # don't fail on first-ever run

    threshold = threshold_pct / 100.0
    failures: list[tuple[str, float]] = []
    compared = 0

    for cur_path in glob.glob(f"{CRITERION_DIR}/**/current/estimates.json", recursive=True):
        rel = os.path.relpath(cur_path, CRITERION_DIR)
        name = rel.split(os.sep)[0]
        main_path = os.path.join(BASELINE_DIR, rel.replace("/current/", "/main/"))
        if not os.path.exists(main_path):
            print(f"  (no baseline for {name}, skipping)")
            continue
        cur_mean = _load_mean(cur_path)
        main_mean = _load_mean(main_path)
        change = (cur_mean - main_mean) / main_mean
        arrow = "▲" if change > 0 else "▼"
        print(f"  {name}: {arrow}{abs(change) * 100:.1f}%")
        if change > threshold:
            failures.append((name, change))
        compared += 1

    if compared == 0:
        print("  (no benchmarks to compare)")
        return True

    if failures:
        print(f"\nRegressions exceeding {threshold_pct:.0f}% threshold:")
        for name, change in failures:
            print(f"  FAIL  {name}: +{change * 100:.1f}% slower than baseline")
        return False

    print("All benchmarks within threshold.")
    return True


if __name__ == "__main__":
    if "--save" in sys.argv:
        save()
    else:
        threshold = float(os.getenv("REGRESSION_THRESHOLD", "20"))
        if not check(threshold):
            sys.exit(1)
