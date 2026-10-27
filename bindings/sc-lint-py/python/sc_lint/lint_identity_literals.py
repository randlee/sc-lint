#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import sys

from sc_lint.lint_common import CHAR_LITERAL_RE
from sc_lint.lint_common import LintDirectivePolicy
from sc_lint.lint_common import decode_rust_literal
from sc_lint.lint_common import discover_repo_root
from sc_lint.lint_common import iter_string_literal_contents
from sc_lint.lint_common import iter_workspace_rust_files
from sc_lint.lint_common import is_code_line
from sc_lint.lint_common import line_is_suppressed
from sc_lint.lint_common import ConfigError
from sc_lint.lint_common import resolve_table
from sc_lint.lint_common import table_missing_error
from sc_lint.lint_common import monotonic_now
from sc_lint.lint_common import rust_file_test_scope
from sc_lint.python_adapter import AdapterError
from sc_lint.python_adapter import error_payload
from sc_lint.python_adapter import success_payload
from sc_lint.python_adapter import write_json as write_adapter_json
from sc_lint.view_common import relative_artifact_path
from sc_lint.view_common import reset_findings_dir
from sc_lint.view_common import write_json
from sc_lint.view_common import write_text


TOOL_NAME = "identity-literals"
DIRECTIVE_POLICY = LintDirectivePolicy(
    tool_key=TOOL_NAME,
    aliases=("rule-008", "rule-009"),
)


def _skip_rust_trivia(source: str, index: int) -> int:
    while index < len(source):
        if source[index].isspace():
            index += 1
        elif source.startswith("//", index):
            newline = source.find("\n", index + 2)
            if newline == -1:
                return len(source)
            index = newline + 1
        elif source.startswith("/*", index):
            depth = 1
            index += 2
            while index < len(source) and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            if depth:
                return len(source)
        else:
            break
    return index


def _read_rust_string_literal(source: str, index: int) -> tuple[int, str] | None:
    if index >= len(source):
        return None
    if source[index] == '"':
        cursor = index + 1
        while cursor < len(source):
            if source[cursor] == "\\":
                cursor += 2
            elif source[cursor] == '"':
                return cursor + 1, decode_rust_literal(source[index + 1 : cursor])
            else:
                cursor += 1
        return None

    if source[index] != "r":
        return None
    cursor = index + 1
    while cursor < len(source) and source[cursor] == "#":
        cursor += 1
    if cursor >= len(source) or source[cursor] != '"':
        return None
    hashes = source[index + 1 : cursor]
    closing = '"' + hashes
    end = source.find(closing, cursor + 1)
    if end == -1:
        return None
    return end + len(closing), source[cursor + 1 : end]


def _split_concat_arguments(source: str, open_paren: int) -> tuple[list[str], int] | None:
    arguments: list[str] = []
    argument_start = open_paren + 1
    cursor = argument_start
    delimiters = [")"]
    while cursor < len(source):
        if source.startswith(("//", "/*"), cursor):
            cursor = _skip_rust_trivia(source, cursor)
            continue
        string_literal = _read_rust_string_literal(source, cursor)
        if string_literal is not None:
            cursor = string_literal[0]
            continue
        char_literal = CHAR_LITERAL_RE.match(source, cursor)
        if char_literal is not None:
            cursor = char_literal.end()
            continue

        character = source[cursor]
        if character in "([{":
            delimiters.append({"(": ")", "[": "]", "{": "}"}[character])
        elif character in ")]}":
            if not delimiters or character != delimiters[-1]:
                return None
            delimiters.pop()
            if not delimiters:
                arguments.append(source[argument_start:cursor])
                if len(arguments) == 1 and not arguments[0].strip():
                    return [], cursor + 1
                return arguments, cursor + 1
        elif character == "," and len(delimiters) == 1:
            arguments.append(source[argument_start:cursor])
            argument_start = cursor + 1
        cursor += 1
    return None


def _single_string_argument(argument: str) -> str | None:
    start = _skip_rust_trivia(argument, 0)
    literal = _read_rust_string_literal(argument, start)
    if literal is None:
        return None
    end, value = literal
    if _skip_rust_trivia(argument, end) != len(argument):
        return None
    return value


def _iter_literal_concat_values(source: str):
    """Yield literal-only `concat!` results without evaluating Rust macros."""
    cursor = 0
    while cursor < len(source):
        if source.startswith(("//", "/*"), cursor):
            cursor = _skip_rust_trivia(source, cursor)
            continue
        string_literal = _read_rust_string_literal(source, cursor)
        if string_literal is not None:
            cursor = string_literal[0]
            continue
        char_literal = CHAR_LITERAL_RE.match(source, cursor)
        if char_literal is not None:
            cursor = char_literal.end()
            continue

        character = source[cursor]
        if character.isalpha() or character == "_":
            end = cursor + 1
            while end < len(source) and (source[end].isalnum() or source[end] == "_"):
                end += 1
            if source[cursor:end] == "concat":
                bang = _skip_rust_trivia(source, end)
                if bang < len(source) and source[bang] == "!":
                    open_paren = _skip_rust_trivia(source, bang + 1)
                    if open_paren < len(source) and source[open_paren] == "(":
                        split = _split_concat_arguments(source, open_paren)
                        if split is not None:
                            arguments, _end = split
                            if arguments and _skip_rust_trivia(arguments[-1], 0) == len(arguments[-1]):
                                arguments.pop()
                            values = [_single_string_argument(argument) for argument in arguments]
                            if values and all(value is not None for value in values):
                                yield (
                                    cursor,
                                    "".join(value for value in values if value is not None),
                                    len(arguments) == 1,
                                )
            cursor = end
            continue
        cursor += 1


@dataclass(frozen=True)
class IdentityViolation:
    path: str
    line_number: int
    line: str
    kind: str

    def render(self) -> str:
        return f"{self.path}:{self.line_number}: {self.line}"


def load_forbidden_literals(repo_root: Path, config_path: str | None = None) -> tuple[str, ...]:
    config, _ = resolve_table("identities", Path(config_path) if config_path else None, repo_root)
    literals = config.get("forbidden_literals", [])
    if not isinstance(literals, list) or not all(isinstance(item, str) for item in literals):
        raise AdapterError("config", "[identities].forbidden_literals must be an array of strings")
    return tuple(literals)


def load_production_canonical_literals(
    repo_root: Path,
    config_path: str | None = None,
) -> dict[str, tuple[str, ...]]:
    config, _ = resolve_table("identities", Path(config_path) if config_path else None, repo_root)
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
        source = "\n".join(lines)
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

        for offset, concatenated, is_single_literal in _iter_literal_concat_values(source):
            if is_single_literal:
                continue
            line_number = source.count("\n", 0, offset) + 1
            line = lines[line_number - 1]
            if not is_code_line(line) or line_is_suppressed(line_number, lines, DIRECTIVE_POLICY):
                continue
            in_test_scope = scope[line_number - 1]
            if in_test_scope:
                if concatenated in forbidden_literals:
                    violations.append(
                        IdentityViolation(
                            path=rel_path.as_posix(),
                            line_number=line_number,
                            line=line.strip(),
                            kind="test_scope_forbidden_literal",
                        )
                    )
                continue

            if (
                concatenated in production_canonical_literals
                and rel_path.as_posix() not in production_canonical_literals[concatenated]
            ):
                violations.append(
                    IdentityViolation(
                        path=rel_path.as_posix(),
                        line_number=line_number,
                        line=line.strip(),
                        kind="production_scope_canonical_literal",
                    )
                )
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
    except (AdapterError, ConfigError) as caught:
        error = table_missing_error(caught) if isinstance(caught, ConfigError) else caught
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
