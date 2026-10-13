#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import argparse
import json
import subprocess
import sys

from sc_lint._binary import binary_path
from sc_lint.lint_common import discover_repo_root


RULE_ID = "SCB-BOUNDARY-004"


def delegation_argv(repo_root: Path) -> list[str]:
    return [
        binary_path(),
        "--json",
        "--root",
        str(repo_root),
        "lint",
        "sc-boundary",
        "--rule-filter",
        RULE_ID,
    ]


def rust_report(repo_root: Path) -> tuple[int, dict]:
    completed = subprocess.run(
        delegation_argv(repo_root),
        check=False,
        capture_output=True,
        text=True,
    )
    payload = completed.stdout or completed.stderr
    try:
        return completed.returncode, json.loads(payload)
    except json.JSONDecodeError as error:
        raise RuntimeError(
            f"sc-lint did not return JSON for {RULE_ID}: {payload.strip()}"
        ) from error


def validate_inventory(repo_root: Path) -> list[str]:
    _, report = rust_report(repo_root)
    findings = report.get("data", report).get("findings", [])
    return [str(finding["message"]) for finding in findings]


def run(repo_root: Path) -> int:
    exit_code, report = rust_report(repo_root)
    status = report.get("data", report).get("status")
    if status in {"pass", "fail"}:
        return 0 if status == "pass" else 1
    return exit_code


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Delegate boundary inventory semantic validation to sc-lint."
    )
    parser.add_argument("--root", help="Repo root to inspect.")
    args = parser.parse_args(argv[1:])
    return run(discover_repo_root(args.root))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
