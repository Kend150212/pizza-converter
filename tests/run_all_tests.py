#!/usr/bin/env python3
"""
Pizza Converter — Comprehensive 4-Tier Test Runner
Executes Tiers 1-4 opaque-box tests covering all 23 features in PROJECT.md.
"""

import sys
import os
import unittest
import time
import json
import argparse
from datetime import datetime, timezone

# Ensure project root and tests/ are in path
TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(TESTS_DIR)
if TESTS_DIR not in sys.path:
    sys.path.insert(0, TESTS_DIR)
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

from test_tier1_features import TestTier1FeatureCoverage
from test_tier2_boundaries import TestTier2Boundaries
from test_tier3_interactions import TestTier3Interactions
from test_tier4_scenarios import TestTier4Scenarios

def run_suite(tiers=None, verbose=False, json_output=False):
    suite = unittest.TestSuite()
    loader = unittest.TestLoader()

    tier_map = {
        1: ("Tier 1: Feature Coverage", loader.loadTestsFromTestCase(TestTier1FeatureCoverage)),
        2: ("Tier 2: Boundary & Corner Cases", loader.loadTestsFromTestCase(TestTier2Boundaries)),
        3: ("Tier 3: Cross-Feature Interactions", loader.loadTestsFromTestCase(TestTier3Interactions)),
        4: ("Tier 4: Real-World Scenarios", loader.loadTestsFromTestCase(TestTier4Scenarios))
    }

    selected_tiers = tiers if tiers else [1, 2, 3, 4]

    print("\n" + "=" * 70)
    print("  PIZZA CONVERTER — 4-TIER REQUIREMENT-DRIVEN E2E TEST RUNNER")
    print("=" * 70)
    print(f"Timestamp: {datetime.now(timezone.utc).isoformat()}")
    print(f"Target Directory: {PROJECT_ROOT}")
    print(f"Active Tiers: {selected_tiers}\n")

    overall_start = time.perf_counter()
    tier_results = []
    total_run = 0
    total_failures = 0
    total_errors = 0
    total_skipped = 0

    for tier_num in selected_tiers:
        tier_name, test_group = tier_map[tier_num]
        print(f"--> Running {tier_name} ({test_group.countTestCases()} test cases)...")

        runner = unittest.TextTestRunner(verbosity=2 if verbose else 1, stream=sys.stdout)
        start_time = time.perf_counter()
        result = runner.run(test_group)
        duration = time.perf_counter() - start_time

        t_run = result.testsRun
        t_fail = len(result.failures)
        t_err = len(result.errors)
        t_skip = len(result.skipped)

        total_run += t_run
        total_failures += t_fail
        total_errors += t_err
        total_skipped += t_skip

        tier_status = "PASS" if (t_fail == 0 and t_err == 0) else "FAIL"

        tier_results.append({
            "tier": tier_num,
            "name": tier_name,
            "status": tier_status,
            "run": t_run,
            "passed": t_run - t_fail - t_err,
            "failures": t_fail,
            "errors": t_err,
            "skipped": t_skip,
            "duration_sec": round(duration, 3),
            "failure_details": [
                {"test": str(f[0]), "trace": f[1]} for f in result.failures + result.errors
            ]
        })
        print(f"    Result: {tier_status} | Run: {t_run} | Passed: {t_run - t_fail - t_err} | Failures: {t_fail} | Errors: {t_err} | Time: {duration:.3f}s\n")

    total_duration = time.perf_counter() - overall_start
    overall_status = "PASSED" if (total_failures == 0 and total_errors == 0) else "FAILED"

    print("=" * 70)
    print("  TEST SUITE SUMMARY MATRIX")
    print("=" * 70)
    print(f"{'Tier':<35} | {'Status':<8} | {'Run':<6} | {'Passed':<6} | {'Failed':<6} | {'Time (s)':<8}")
    print("-" * 75)
    for r in tier_results:
        print(f"{r['name']:<35} | {r['status']:<8} | {r['run']:<6} | {r['passed']:<6} | {r['failures'] + r['errors']:<6} | {r['duration_sec']:<8.3f}")
    print("-" * 75)
    print(f"{'TOTAL / OVERALL':<35} | {overall_status:<8} | {total_run:<6} | {total_run - total_failures - total_errors:<6} | {total_failures + total_errors:<6} | {total_duration:<8.3f}")
    print("=" * 70 + "\n")

    report_data = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "overall_status": overall_status,
        "total_run": total_run,
        "total_passed": total_run - total_failures - total_errors,
        "total_failures": total_failures,
        "total_errors": total_errors,
        "total_skipped": total_skipped,
        "total_duration_sec": round(total_duration, 3),
        "tiers": tier_results
    }

    report_path = os.path.join(TESTS_DIR, "test_results.json")
    try:
        with open(report_path, "w", encoding="utf-8") as f:
            json.dump(report_data, f, indent=2)
        print(f"Structured test report written to: {report_path}")
    except Exception as e:
        print(f"Warning: could not write test_results.json: {e}")

    return 0 if overall_status == "PASSED" else 1

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Pizza Converter 4-Tier Test Runner")
    parser.add_argument("--tier", type=int, choices=[1, 2, 3, 4], help="Run only specific tier (1, 2, 3, or 4)")
    parser.add_argument("-v", "--verbose", action="store_true", help="Verbose test runner output")
    parser.add_argument("--json", action="store_true", help="Output summary in json")

    args = parser.parse_args()
    selected_tiers = [args.tier] if args.tier else None
    exit_code = run_suite(tiers=selected_tiers, verbose=args.verbose, json_output=args.json)
    sys.exit(exit_code)
