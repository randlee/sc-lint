#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import sys

from sc_lint.lint_common import ConfigError
from sc_lint.lint_common import discover_repo_root
from sc_lint.lint_common import is_code_line
from sc_lint.lint_common import iter_workspace_rust_files
from sc_lint.lint_common import monotonic_now
from sc_lint.lint_common import resolve_table
from sc_lint.lint_common import rust_file_test_scope
from sc_lint.lint_common import table_missing_error
from sc_lint.python_adapter import AdapterError
from sc_lint.python_adapter import error_payload
from sc_lint.python_adapter import success_payload
from sc_lint.python_adapter import write_json


SLEEP_RE = re.compile(r"\b(?:(?:std::)?thread|tokio::time)::sleep\s*\(")


@dataclass(frozen=True)
class FixedSleepViolation:
    path: str
    line_number: int
    line: str

    def render(self) -> str:
        return f"{self.path}:{self.line_number}: {self.line}"


def load_allowed_paths(repo_root: Path, config_path: str | None = None) -> tuple[str, ...]:
    config, _ = resolve_table("fixed_sleep", Path(config_path) if config_path else None, repo_root)
    allowed_paths = config.get("allowed_paths", [])
    if not isinstance(allowed_paths, list) or not all(
        isinstance(item, str) and item.strip() for item in allowed_paths
    ):
        raise AdapterError("config", "[fixed_sleep].allowed_paths must be an array of non-empty path strings")
    return tuple(allowed_paths)


def collect_fixed_sleep_violations(
    repo_root: Path,
    *,
    allowed_paths: tuple[str, ...],
) -> list[FixedSleepViolation]:
    allowed_path_set = set(allowed_paths)
    violations: list[FixedSleepViolation] = []
    for source in iter_workspace_rust_files(repo_root):
        relative = source.relative_to(repo_root)
        relative_path = relative.as_posix()
        lines = source.read_text(encoding="utf-8").splitlines()
        test_scope = rust_file_test_scope(relative, lines)
        for line_number, (line, in_test_scope) in enumerate(zip(lines, test_scope), start=1):
            if not in_test_scope or relative_path in allowed_path_set or not is_code_line(line):
                continue
            if SLEEP_RE.search(line):
                violations.append(FixedSleepViolation(relative_path, line_number, line.strip()))
    return violations


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Reject fixed thread::sleep calls in Rust test scope.")
    parser.add_argument("--root", help="Repo root to inspect.")
    parser.add_argument("--config", help="Repo config file override.")
    parser.add_argument("--json", action="store_true")
    return parser.parse_args(argv[1:])


def main(argv: list[str]) -> int:
    args: argparse.Namespace | None = None
    try:
        args = parse_args(argv)
        repo_root = discover_repo_root(args.root)
        allowed_paths = load_allowed_paths(repo_root, args.config)
        started = monotonic_now()
        violations = collect_fixed_sleep_violations(repo_root, allowed_paths=allowed_paths)
        elapsed_ms = int((monotonic_now() - started) * 1000)
        findings = [violation.render() for violation in violations]
        summary = (
            "fixed sleep usage found in Rust test scope"
            if findings
            else "no fixed sleep usage found in Rust test scope"
        )
        data = {
            "tool": "sc-lint-fixed-sleep",
            "status": "fail" if findings else "pass",
            "summary": summary,
            "allowed_paths": list(allowed_paths),
            "findings": findings,
            "elapsed_ms": elapsed_ms,
        }
        if args.json:
            write_json(success_payload(summary=summary, data=data))
        else:
            print(f"fixed-sleep {'failed' if findings else 'passed'}")
            for finding in findings:
                print(finding)
        return 1 if findings else 0
    except (AdapterError, ConfigError) as caught:
        error = table_missing_error(caught) if isinstance(caught, ConfigError) else caught
        if args is not None and args.json:
            write_json(error_payload(error))
        else:
            print(error.message, file=sys.stderr)
        return 3 if error.kind == "config" else 1
    except Exception as error:  # pragma: no cover - contract guardrail
        payload = error_payload(
            AdapterError("backend_protocol", f"failed to evaluate fixed-sleep: {error}", details={"tool": "sc-lint-fixed-sleep"})
        )
        if args is not None and args.json:
            write_json(payload)
        else:
            print(payload["error"]["message"], file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
