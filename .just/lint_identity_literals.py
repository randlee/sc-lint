#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import sys

from lint_common import LintDirectivePolicy
from lint_common import discover_repo_root
from lint_common import iter_string_literal_contents
from lint_common import iter_workspace_rust_files
from lint_common import is_code_line
from lint_common import line_is_suppressed
from lint_common import load_lint_config
from lint_common import monotonic_now
from lint_common import rust_file_test_scope
from python_adapter import AdapterError
from python_adapter import error_payload
from python_adapter import success_payload
from python_adapter import write_json as write_adapter_json
from view_common import relative_artifact_path
from view_common import reset_findings_dir
from view_common import write_json
from view_common import write_text


TOOL_NAME = "identity-literals"
DIRECTIVE_POLICY = LintDirectivePolicy(
    tool_key=TOOL_NAME,
    aliases=("rule-008", "rule-009"),
)


@dataclass(frozen=True)
class IdentityViolation:
    path: str
    line_number: int
    line: str
    kind: str

    def render(self) -> str:
        return f"{self.path}:{self.line_number}: {self.line}"


def load_forbidden_literals(repo_root: Path, config_path: str | None = None) -> tuple[str, ...]:
    config = load_lint_config(repo_root, config_path).get("identities", {})
    if not isinstance(config, dict):
        raise AdapterError("config", "[identities] must be a TOML table")
    literals = config.get("forbidden_literals", [])
    if not isinstance(literals, list) or not all(isinstance(item, str) for item in literals):
        raise AdapterError("config", "[identities].forbidden_literals must be an array of strings")
    return tuple(literals)


def load_production_canonical_literals(
    repo_root: Path,
    config_path: str | None = None,
) -> dict[str, tuple[str, ...]]:
    config = load_lint_config(repo_root, config_path).get("identities", {})
    if not isinstance(config, dict):
        raise AdapterError("config", "[identities] must be a TOML table")
    literal_map = config.get("production_canonical_literals", {})
    if not isinstance(literal_map, dict):
        raise AdapterError("config", "[identities].production_canonical_literals must be a TOML table")

    canonical_literals: dict[str, tuple[str, ...]] = {}
    for literal, allowed_paths in literal_map.items():
        if not isinstance(literal, str):
            raise AdapterError("config", "[identities].production_canonical_literals keys must be strings")
        if not isinstance(allowed_paths, list) or not all(isinstance(item, str) for item in allowed_paths):
            raise AdapterError(
                "config",
                "[identities].production_canonical_literals values must be arrays of path strings",
            )
        canonical_literals[literal] = tuple(allowed_paths)
    return canonical_literals


def collect_identity_violations(
    repo_root: Path,
    *,
    forbidden_literals: tuple[str, ...],
    production_canonical_literals: dict[str, tuple[str, ...]],
) -> list[IdentityViolation]:
    violations: list[IdentityViolation] = []
    for abs_path in iter_workspace_rust_files(repo_root):
        rel_path = abs_path.relative_to(repo_root)
        lines = abs_path.read_text(encoding="utf-8").splitlines()
        scope = rust_file_test_scope(rel_path, lines)

        for line_number, (line, in_test_scope) in enumerate(zip(lines, scope, strict=True), start=1):
            if not is_code_line(line):
                continue
            if line_is_suppressed(line_number, lines, DIRECTIVE_POLICY):
                continue
            literal_contents = set(iter_string_literal_contents(line))
            if in_test_scope:
                if not any(literal in literal_contents for literal in forbidden_literals):
                    continue
                violations.append(
                    IdentityViolation(
                        path=rel_path.as_posix(),
                        line_number=line_number,
                        line=line.strip(),
                        kind="test_scope_forbidden_literal",
                    )
                )
                continue

            for literal, allowed_paths in production_canonical_literals.items():
                if literal not in literal_contents:
                    continue
                if rel_path.as_posix() in allowed_paths:
                    continue
                violations.append(
                    IdentityViolation(
                        path=rel_path.as_posix(),
                        line_number=line_number,
                        line=line.strip(),
                        kind="production_scope_canonical_literal",
                    )
                )
                break
    return violations


def build_data(
    repo_root: Path,
    violations: list[IdentityViolation],
    forbidden_literals: tuple[str, ...],
    production_canonical_literals: dict[str, tuple[str, ...]],
    elapsed_ms: int,
) -> dict:
    output_dir = reset_findings_dir(repo_root, TOOL_NAME)
    findings = [violation.render() for violation in violations]
    data = {
        "tool": f"sc-lint-{TOOL_NAME}",
        "status": "pass" if not violations else "fail",
        "summary": (
            "no disallowed raw production literals found in Rust code"
            if not violations
            else "raw production literals found in Rust code"
        ),
        "forbidden_literals": list(forbidden_literals),
        "production_canonical_literals": {
            literal: list(paths) for literal, paths in production_canonical_literals.items()
        },
        "findings": findings,
        "violation_kinds": [violation.kind for violation in violations],
        "elapsed_ms": elapsed_ms,
    }
    write_json(output_dir / "summary.json", data)
    lines = [data["summary"]]
    if findings:
        lines.extend(["findings:", *findings])
    else:
        lines.append("findings: none")
    write_text(output_dir / "summary.txt", "\n".join(lines) + "\n")
    data["artifact_dir"] = relative_artifact_path(repo_root, output_dir)
    return data


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Reject forbidden test literals and duplicated canonical production literals in Rust code."
    )
    parser.add_argument("--root", help="Repo root to inspect.")
    parser.add_argument("--config", help="Repo config file override.")
    parser.add_argument("--json", action="store_true")
    return parser.parse_args(argv[1:])


def main(argv: list[str]) -> int:
    try:
        args = parse_args(argv)
        repo_root = discover_repo_root(args.root)
        forbidden_literals = load_forbidden_literals(repo_root, args.config)
        production_canonical_literals = load_production_canonical_literals(repo_root, args.config)
        started = monotonic_now()
        violations = collect_identity_violations(
            repo_root,
            forbidden_literals=forbidden_literals,
            production_canonical_literals=production_canonical_literals,
        )
        elapsed_ms = int((monotonic_now() - started) * 1000)
        data = build_data(
            repo_root,
            violations,
            forbidden_literals,
            production_canonical_literals,
            elapsed_ms,
        )
        if args.json:
            write_adapter_json(success_payload(summary=data["summary"], data=data))
            return 0 if not violations else 1

        print(f"identity-literals {'passed' if not violations else 'failed'}")
        if violations:
            for finding in data["findings"]:
                print(finding)
        return 0 if not violations else 1
    except AdapterError as error:
        if args.json:
            write_adapter_json(error_payload(error))
            return 3 if error.kind == "config" else 1
        print(error.message, file=sys.stderr)
        return 3 if error.kind == "config" else 1
    except Exception as error:  # pragma: no cover - contract guardrail
        payload = error_payload(
            AdapterError(
                "backend_protocol",
                f"failed to evaluate identity literals: {error}",
                details={"tool": f"sc-lint-{TOOL_NAME}"},
            )
        )
        if "args" in locals() and args.json:
            write_adapter_json(payload)
            return 1
        print(payload["error"]["message"], file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
