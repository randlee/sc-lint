#!/usr/bin/env python3
"""Enforce bounded Rust function size using non-comment code lines."""
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import sys

from sc_lint.lint_common import discover_repo_root
from sc_lint.lint_common import iter_workspace_rust_files
from sc_lint.lint_common import monotonic_now
from sc_lint.python_adapter import AdapterError
from sc_lint.python_adapter import error_payload
from sc_lint.python_adapter import success_payload
from sc_lint.python_adapter import write_json as write_adapter_json
from sc_lint.view_common import relative_artifact_path
from sc_lint.view_common import reset_findings_dir
from sc_lint.view_common import write_json
from sc_lint.view_common import write_text


TOOL_NAME = "function-length"
WARN_THRESHOLD = 70
FAIL_THRESHOLD = 80
FUNCTION_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:(?:async|const|unsafe|extern)\s+)*fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b"
)
SC_LINT_ATTRIBUTE_RE = re.compile(
    r"^\s*#\s*\[\s*sc_lint\s*\((?P<body>.*)\)\s*\]\s*$", re.DOTALL
)
FUNCTION_LENGTH_DIRECTIVE_RE = re.compile(
    r"(?:^|,)\s*function_length\s*\.\s*fail_at\s*\(\s*(?P<limit>[1-9][0-9]*)\s*\)\s*(?=,|$)",
    re.DOTALL,
)


@dataclass(frozen=True)
class FunctionSpan:
    path: Path
    name: str
    start_line: int
    end_line: int
    code_lines: int
    fail_threshold: int

    @property
    def is_hard_violation(self) -> bool:
        return self.code_lines >= self.fail_threshold


@dataclass(frozen=True)
class Classification:
    advisories: tuple[FunctionSpan, ...]
    failures: tuple[FunctionSpan, ...]


def mask_rust_non_code(source: str) -> str:
    """Replace comments and literals with spaces while preserving lines and offsets."""
    output = list(source)
    index = 0
    block_depth = 0
    length = len(source)

    def blank(start: int, end: int) -> None:
        for position in range(start, end):
            if output[position] != "\n":
                output[position] = " "

    while index < length:
        if block_depth:
            if source.startswith("/*", index):
                blank(index, index + 2)
                block_depth += 1
                index += 2
            elif source.startswith("*/", index):
                blank(index, index + 2)
                block_depth -= 1
                index += 2
            else:
                blank(index, index + 1)
                index += 1
            continue

        if source.startswith("//", index):
            end = source.find("\n", index)
            blank(index, length if end == -1 else end)
            index = length if end == -1 else end
            continue
        if source.startswith("/*", index):
            blank(index, index + 2)
            block_depth = 1
            index += 2
            continue

        raw_match = re.match(r"(?:br|r)(?P<hashes>#{0,255})\"", source[index:])
        if raw_match:
            delimiter = '\"' + raw_match.group("hashes")
            content_start = index + raw_match.end()
            closing = source.find(delimiter, content_start)
            end = length if closing == -1 else closing + len(delimiter)
            blank(index, end)
            index = end
            continue

        if source[index] == '\"':
            end = index + 1
            while end < length:
                if source[end] == "\\":
                    end += 2
                elif source[end] == '\"':
                    end += 1
                    break
                else:
                    end += 1
            blank(index, min(end, length))
            index = min(end, length)
            continue

        if source[index] == "'":
            char_match = re.match(r"'(?:\\.|[^'\\\n])'", source[index:])
            if char_match:
                end = index + char_match.end()
                blank(index, end)
                index = end
                continue

        index += 1
    return "".join(output)


def is_test_only_path(path: Path) -> bool:
    if "tests" in path.parts:
        return True
    name = path.name
    return name == "tests.rs" or name.startswith("test_") or name.endswith(("_test.rs", "_tests.rs")) or "test_support" in name


def preceding_attributes(lines: list[str], function_index: int) -> list[str]:
    attributes: list[str] = []
    index = function_index - 1
    while index >= 0:
        if not lines[index].strip():
            break
        attribute_lines: list[str] = []
        while index >= 0:
            attribute_lines.append(lines[index])
            if lines[index].lstrip().startswith("#["):
                attributes.append("\n".join(reversed(attribute_lines)))
                index -= 1
                break
            index -= 1
        else:
            break
    return attributes


def function_override(attributes: list[str], path: Path, line_number: int) -> int | None:
    for attribute in attributes:
        match = SC_LINT_ATTRIBUTE_RE.match(attribute)
        if match is None:
            continue
        directives = list(FUNCTION_LENGTH_DIRECTIVE_RE.finditer(match.group("body")))
        if len(directives) == 1:
            return int(directives[0].group("limit"))
        if "function_length" in match.group("body"):
            raise AdapterError(
                "config",
                f"{path}:{line_number}: expected exactly one function_length.fail_at(N) directive with positive integer N",
            )
    return None


def find_function_spans(path: Path) -> list[FunctionSpan]:
    source = path.read_text(encoding="utf-8")
    raw_lines = source.splitlines()
    masked_lines = mask_rust_non_code(source).splitlines()
    spans: list[FunctionSpan] = []
    index = 0
    while index < len(masked_lines):
        match = FUNCTION_RE.match(masked_lines[index])
        if match is None:
            index += 1
            continue
        attributes = preceding_attributes(raw_lines, index)
        if any("test" in attribute.lower() for attribute in attributes):
            index += 1
            continue
        override = function_override(attributes, path, index + 1)

        signature_end = index
        while signature_end < len(masked_lines):
            line = masked_lines[signature_end]
            brace = line.find("{")
            semicolon = line.find(";")
            if semicolon != -1 and (brace == -1 or semicolon < brace):
                break
            if brace != -1:
                break
            signature_end += 1
        if signature_end >= len(masked_lines) or "{" not in masked_lines[signature_end]:
            index += 1
            continue

        depth = 0
        end_index: int | None = None
        for candidate in range(signature_end, len(masked_lines)):
            for character in masked_lines[candidate]:
                if character == "{":
                    depth += 1
                elif character == "}":
                    depth -= 1
            if depth == 0:
                end_index = candidate
                break
        if end_index is None:
            index += 1
            continue

        spans.append(
            FunctionSpan(
                path=path,
                name=match.group("name"),
                start_line=index + 1,
                end_line=end_index + 1,
                code_lines=sum(bool(line.strip()) for line in masked_lines[index : end_index + 1]),
                fail_threshold=override or FAIL_THRESHOLD,
            )
        )
        index = end_index + 1
    return spans


def classify(functions: list[FunctionSpan]) -> Classification:
    advisories = tuple(function for function in functions if WARN_THRESHOLD <= function.code_lines < function.fail_threshold)
    failures = tuple(function for function in functions if function.is_hard_violation)
    return Classification(advisories=advisories, failures=failures)


def render(repo_root: Path, function: FunctionSpan) -> str:
    return f"{function.path.relative_to(repo_root)}:{function.start_line}-{function.end_line}: {function.name} ({function.code_lines} code lines; fails at {function.fail_threshold})"


def build_data(repo_root: Path, classification: Classification, elapsed_ms: int) -> dict:
    output_dir = reset_findings_dir(repo_root, TOOL_NAME)
    findings = [
        {"severity": severity, "path": function.path.relative_to(repo_root).as_posix(), "name": function.name, "start_line": function.start_line, "end_line": function.end_line, "code_lines": function.code_lines, "fail_threshold": function.fail_threshold}
        for severity, functions in (("advisory", classification.advisories), ("error", classification.failures))
        for function in functions
    ]
    data = {
        "tool": f"sc-lint-{TOOL_NAME}",
        "status": "fail" if classification.failures else "pass",
        "summary": "function-length limits satisfied" if not classification.failures else "function-length limits exceeded",
        "thresholds": {"advisory": WARN_THRESHOLD, "default_fail_at": FAIL_THRESHOLD, "counting": "non-comment, non-whitespace Rust code lines"},
        "findings": findings,
        "elapsed_ms": elapsed_ms,
    }
    write_json(output_dir / "summary.json", data)
    lines = [data["summary"], "counting: non-comment, non-whitespace Rust code lines"]
    for label, functions in (("advisories", classification.advisories), ("failures", classification.failures)):
        if functions:
            lines.append(f"{label}:")
            lines.extend(render(repo_root, function) for function in functions)
    if not classification.advisories and not classification.failures:
        lines.append("findings: none")
    write_text(output_dir / "summary.txt", "\n".join(lines) + "\n")
    data["artifact_dir"] = relative_artifact_path(repo_root, output_dir)
    return data


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Enforce Rust function length using non-comment code lines.")
    parser.add_argument("--root", help="Repo root to inspect.")
    parser.add_argument("--config", help="Accepted for Python-adapter compatibility.")
    parser.add_argument("--json", action="store_true")
    return parser.parse_args(argv[1:])


def main(argv: list[str]) -> int:
    try:
        args = parse_args(argv)
        repo_root = discover_repo_root(args.root)
        started = monotonic_now()
        functions = [function for path in iter_workspace_rust_files(repo_root) if not is_test_only_path(path.relative_to(repo_root)) for function in find_function_spans(path)]
        classification = classify(functions)
        data = build_data(repo_root, classification, int((monotonic_now() - started) * 1000))
        if args.json:
            write_adapter_json(success_payload(summary=data["summary"], data=data))
        else:
            print(data["summary"])
            for function in classification.advisories:
                print(f"advisory: {render(repo_root, function)}")
            for function in classification.failures:
                print(f"error: {render(repo_root, function)}")
        return 1 if classification.failures else 0
    except AdapterError as error:
        if "args" in locals() and args.json:
            write_adapter_json(error_payload(error))
        else:
            print(error.message, file=sys.stderr)
        return 3 if error.kind == "config" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
