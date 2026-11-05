#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
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


TOOL_NAME = "token-boundary"


@dataclass(frozen=True)
class BoundaryViolation:
    path: str
    line_number: int
    line: str
    kind: str

    def render(self) -> str:
        return f"{self.path}:{self.line_number}: [{self.kind}] {self.line}"


@dataclass(frozen=True)
class TokenBoundaryConfig:
    tokens: tuple[str, ...]
    allowed_paths: tuple[str, ...]
    required_positive: tuple[tuple[str, str], ...]


def _string_list(config: dict, key: str) -> tuple[str, ...]:
    values = config.get(key)
    if not isinstance(values, list) or not all(isinstance(value, str) and value.strip() for value in values):
        raise AdapterError("config", f"[token_boundary].{key} must be an array of non-empty strings")
    return tuple(values)


def load_config(repo_root: Path, config_path: str | None = None) -> TokenBoundaryConfig:
    config, _ = resolve_table("token_boundary", Path(config_path) if config_path else None, repo_root)
    tokens = _string_list(config, "tokens")
    if not tokens:
        raise AdapterError("config", "[token_boundary].tokens must contain at least one token")
    allowed_paths = _string_list(config, "allowed_paths")
    entries = config.get("required_positive")
    if not isinstance(entries, list):
        raise AdapterError("config", "[token_boundary].required_positive must be an array of tables")

    required_positive: list[tuple[str, str]] = []
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            raise AdapterError("config", f"[token_boundary].required_positive[{index}] must be a table")
        path = entry.get("path")
        symbol = entry.get("symbol")
        if not isinstance(path, str) or not path.strip() or not isinstance(symbol, str) or not symbol.strip():
            raise AdapterError(
                "config",
                f"[token_boundary].required_positive[{index}] needs non-empty path and symbol strings",
            )
        required_positive.append((path, symbol))
    return TokenBoundaryConfig(tokens, allowed_paths, tuple(required_positive))


def _is_test_source_path(relative: Path) -> bool:
    if relative.name == "tests.rs" or relative.name.endswith("_tests.rs"):
        return True
    return any(part == "tests" or part.endswith("_tests") for part in relative.parts[:-1])


def collect_violations(repo_root: Path, config: TokenBoundaryConfig) -> list[BoundaryViolation]:
    violations: list[BoundaryViolation] = []
    for required_path, symbol in config.required_positive:
        source = repo_root / required_path
        if not source.is_file() or symbol not in source.read_text(encoding="utf-8"):
            violations.append(BoundaryViolation(required_path, 0, symbol, "required_positive_missing"))

    allowed_path_set = set(config.allowed_paths)
    for source in iter_workspace_rust_files(repo_root):
        relative = source.relative_to(repo_root)
        relative_path = relative.as_posix()
        if relative_path in allowed_path_set or _is_test_source_path(relative):
            continue
        lines = source.read_text(encoding="utf-8").splitlines()
        test_scope = rust_file_test_scope(relative, lines)
        for line_number, (line, in_test_scope) in enumerate(zip(lines, test_scope, strict=True), start=1):
            if in_test_scope or not is_code_line(line):
                continue
            if any(token in line for token in config.tokens):
                violations.append(BoundaryViolation(relative_path, line_number, line.strip(), "source_use_not_allowed"))
    return violations


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check configured token use outside allowed Rust source paths.")
    parser.add_argument("--root", help="Repo root to inspect.")
    parser.add_argument("--config", help="Repo config file override.")
    parser.add_argument("--json", action="store_true")
    return parser.parse_args(argv[1:])


def main(argv: list[str]) -> int:
    args: argparse.Namespace | None = None
    try:
        args = parse_args(argv)
        repo_root = discover_repo_root(args.root)
        config = load_config(repo_root, args.config)
        started = monotonic_now()
        violations = collect_violations(repo_root, config)
        elapsed_ms = int((monotonic_now() - started) * 1000)
        findings = [violation.render() for violation in violations]
        summary = "token boundary satisfied" if not findings else f"token boundary violated ({len(findings)} findings)"
        data = {
            "tool": "sc-lint-token-boundary",
            "status": "pass" if not findings else "fail",
            "summary": summary,
            "tokens": list(config.tokens),
            "allowed_paths": list(config.allowed_paths),
            "findings": findings,
            "elapsed_ms": elapsed_ms,
        }
        if args.json:
            write_json(success_payload(summary=summary, data=data))
        else:
            print(f"token-boundary {'passed' if not findings else 'failed'}")
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
            AdapterError("backend_protocol", f"failed to evaluate token-boundary: {error}", details={"tool": "sc-lint-token-boundary"})
        )
        if args is not None and args.json:
            write_json(payload)
        else:
            print(payload["error"]["message"], file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
