#!/usr/bin/env python3
from __future__ import annotations

from collections import Counter
import importlib.util
from pathlib import Path
import argparse
import sys
import unittest
import subprocess

from sc_lint.lint_common import discover_repo_root


def flatten_suite(suite: unittest.TestSuite) -> list[unittest.TestCase]:
    tests: list[unittest.TestCase] = []
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            tests.extend(flatten_suite(item))
        else:
            tests.append(item)
    return tests


def fixture_name(test: unittest.TestCase) -> str:
    test_id = test.id()
    parts = test_id.split(".")
    if len(parts) >= 3:
        return ".".join(parts[:-1])
    return test_id


def count_fixtures(suite: unittest.TestSuite) -> list[tuple[str, int]]:
    counter: Counter[str] = Counter()
    for test in flatten_suite(suite):
        counter[fixture_name(test)] += 1
    return sorted(counter.items())


def print_fixture_summary(fixture_counts: list[tuple[str, int]]) -> None:
    print("pytests fixture counts:")
    for fixture, count in fixture_counts:
        print(f"  {fixture}: {count}")
    print(f"pytests fixtures total: {len(fixture_counts)}")


def build_suite(tests_dir: Path) -> unittest.TestSuite:
    loader = unittest.defaultTestLoader
    suite = unittest.TestSuite()
    for test_path in sorted(tests_dir.glob("test_*.py")):
        spec = importlib.util.spec_from_file_location(test_path.stem, test_path)
        if spec is None or spec.loader is None:
            raise ImportError(f"unable to load test module from {test_path}")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        suite.addTests(loader.loadTestsFromModule(module))
    return suite


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Run lint-tool Python unit tests with fixture summary output.")
    parser.add_argument("--root", help="Repo root to inspect.")
    args = parser.parse_args(argv[1:])
    repo_root = discover_repo_root(args.root)

    suite = build_suite(Path(__file__).resolve().parent / "tests")
    fixture_counts = count_fixtures(suite)
    print_fixture_summary(fixture_counts)
    runner = unittest.TextTestRunner(stream=sys.stdout, verbosity=1)
    result = runner.run(suite)
    orchestration_tests = [
        repo_root / ".claude/skills/closing-triage/scripts/test_query_open_findings.py",
        repo_root / ".claude/skills/graph-orchestration/scripts/test_validate_findings.py",
        repo_root / ".claude/skills/triaging-findings/tests/test_check_dependencies.py",
        repo_root / ".claude/skills/triaging-findings/tests/test_triage_record_template.py",
        repo_root / ".claude/skills/triaging-findings/tests/test_qa_triage_prompt.py",
        repo_root / ".just/tests/test_team_lead_roster_check.py",
    ]
    pytest = subprocess.run([sys.executable, "-m", "pytest", "-q", *map(str, orchestration_tests)], check=False)
    return 0 if result.wasSuccessful() and pytest.returncode == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
