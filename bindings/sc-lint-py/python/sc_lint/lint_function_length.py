#!/usr/bin/env python3
"""Gate Rust function length against a recorded baseline.

Functions at or over FAIL_THRESHOLD code lines fail the gate only when they are
new or grew past their baseline entry. Functions in the WARN band are advisory.
Allowances ratchet down: a shrunk function is reported stale, and writing the
baseline records its new length. Raising an allowance needs --allow-growth.
Counting ignores comments and whitespace; literal text counts as code.
"""
from __future__ import annotations

import argparse
from dataclasses import asdict
from dataclasses import dataclass
from pathlib import Path
import re
import sys

from sc_lint.lint_common import discover_repo_root
from sc_lint.lint_common import iter_workspace_rust_files
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


TOOL_NAME = "function-length"
WARN_THRESHOLD = 70
FAIL_THRESHOLD = 80
BASELINE_PATH = Path(".just/function-length-baseline.tsv")
FUNCTION_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:(?:async|const|unsafe|extern)\s+)*fn\s+(?P<name>[A-Za-z_]\w*)\b"
)
RAW_STRING_RE = re.compile(r"(?:br|r)(?P<hashes>#*)\"")
CHAR_LITERAL_RE = re.compile(r"'(?:\\u\{[0-9A-Fa-f_]+\}|\\.|[^'\\\n])'")
TEST_ATTRIBUTE_RE = re.compile(r"#\[(?:[A-Za-z_]\w*::)*test(?:\(.*\))?\]|#\[cfg\(test\)\]")

Key = tuple[str, str, int]


@dataclass(frozen=True)
class FunctionSpan:
    path: str
    name: str
    occurrence: int
    start_line: int
    end_line: int
    code_lines: int

    @property
    def key(self) -> Key:
        return (self.path, self.name, self.occurrence)


@dataclass(frozen=True)
class Violation:
    function: FunctionSpan
    recorded_lines: int | None

    @property
    def reason(self) -> str:
        return "new" if self.recorded_lines is None else "grown"


@dataclass(frozen=True)
class Evaluation:
    advisories: tuple[FunctionSpan, ...]
    violations: tuple[Violation, ...]
    stale: tuple[Key, ...]


def is_word(character: str) -> bool:
    return character.isalnum() or character == "_"


def blank(source: str, buffers: tuple[list[str], ...], start: int, end: int) -> None:
    for buffer in buffers:
        for position in range(start, end):
            if source[position] != "\n":
                buffer[position] = " "


def block_comment_end(source: str, start: int) -> int:
    depth, index = 0, start
    while index < len(source):
        if source.startswith("/*", index):
            depth, index = depth + 1, index + 2
        elif source.startswith("*/", index):
            depth, index = depth - 1, index + 2
            if depth == 0:
                return index
        else:
            index += 1
    return len(source)


def literal_end(source: str, index: int) -> int | None:
    """Return the end of a string, raw string, or char literal starting at `index`."""
    raw = RAW_STRING_RE.match(source, index)
    if raw is not None and (index == 0 or not is_word(source[index - 1])):
        closing = source.find('"' + raw.group("hashes"), raw.end())
        return len(source) if closing == -1 else closing + len(raw.group("hashes")) + 1
    if source[index] == '"':
        end = index + 1
        while end < len(source) and source[end] != '"':
            end += 2 if source[end] == "\\" else 1
        return min(end + 1, len(source))
    char = CHAR_LITERAL_RE.match(source, index) if source[index] == "'" else None
    return None if char is None else char.end()


def mask_rust_source(source: str) -> tuple[str, str]:
    """Return (code, structure). Comments are blanked in both; literals only in structure."""
    code, structure = list(source), list(source)
    index = 0
    while index < len(source):
        if source.startswith("//", index):
            end = source.find("\n", index)
            end = len(source) if end == -1 else end
            blank(source, (code, structure), index, end)
        elif source.startswith("/*", index):
            end = block_comment_end(source, index)
            blank(source, (code, structure), index, end)
        elif (end := literal_end(source, index)) is not None:
            blank(source, (structure,), index, end)
        else:
            index += 1
            continue
        index = end
    return "".join(code), "".join(structure)


def collect_attribute(structure_lines: list[str], start: int) -> tuple[str, int]:
    """Return the whitespace-free text of the attribute at `start` and the next line index."""
    depth, index, collected = 0, start, []
    while index < len(structure_lines):
        line = structure_lines[index]
        collected.append(line)
        depth += line.count("[") - line.count("]")
        index += 1
        if depth <= 0:
            break
    return "".join("".join(collected).split()), index


def item_body_end(structure_lines: list[str], start: int) -> int | None:
    """Return the line closing the body that opens at or after `start`, or None if bodiless."""
    braces, nesting, opened = 0, 0, False
    for line_index in range(start, len(structure_lines)):
        for character in structure_lines[line_index]:
            if character in "([":
                nesting += 1
            elif character in ")]":
                nesting -= 1
            elif character == "{":
                braces, opened = braces + 1, True
            elif character == "}":
                braces -= 1
            elif character == ";" and not opened and nesting == 0:
                return None
            if opened and braces == 0:
                return line_index
    return None


def find_functions(source: str, relative_path: Path) -> list[FunctionSpan]:
    code_text, structure_text = mask_rust_source(source)
    code_lines, structure_lines = code_text.split("\n"), structure_text.split("\n")
    test_scope = rust_file_test_scope(Path("/" + relative_path.as_posix()), structure_lines)
    functions: list[FunctionSpan] = []
    occurrences: dict[str, int] = {}
    pending: list[str] = []
    index = 0
    while index < len(structure_lines):
        stripped = structure_lines[index].strip()
        if stripped.startswith(("#[", "#![")):
            normalized, index = collect_attribute(structure_lines, index)
            pending.append(normalized)
            continue
        if not stripped:
            index += 1
            continue
        match = FUNCTION_RE.match(structure_lines[index])
        end = item_body_end(structure_lines, index) if match and not test_scope[index] else None
        if end is None:
            pending = []
            index += 1
            continue
        if not any(TEST_ATTRIBUTE_RE.fullmatch(attribute) for attribute in pending):
            name = match.group("name")
            occurrence = occurrences.get(name, 0)
            occurrences[name] = occurrence + 1
            code_count = sum(bool(line.strip()) for line in code_lines[index : end + 1])
            functions.append(
                FunctionSpan(relative_path.as_posix(), name, occurrence, index + 1, end + 1, code_count)
            )
        pending = []
        index = end + 1
    return functions


def scan_repo(repo_root: Path) -> list[FunctionSpan]:
    functions: list[FunctionSpan] = []
    for path in iter_workspace_rust_files(repo_root):
        functions.extend(find_functions(path.read_text(encoding="utf-8"), path.relative_to(repo_root)))
    return functions


def over_cap_by_key(functions: list[FunctionSpan]) -> dict[Key, FunctionSpan]:
    """Map each over-cap function to its span. Identity is (path, name, occurrence in file)."""
    return {function.key: function for function in functions if function.code_lines >= FAIL_THRESHOLD}


def read_baseline(repo_root: Path) -> dict[Key, int] | None:
    """Return the recorded allowances, or None when the baseline file does not exist."""
    path = repo_root / BASELINE_PATH
    if not path.is_file():
        return None
    entries: dict[Key, int] = {}
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip() or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) != 4 or not parts[2].isdigit() or not parts[3].isdigit():
            raise AdapterError(
                "config",
                f"{BASELINE_PATH.as_posix()}:{number}: expected a path<TAB>name<TAB>occurrence<TAB>code_lines row",
            )
        key = (parts[0], parts[1], int(parts[2]))
        if key in entries:
            raise AdapterError("config", f"{BASELINE_PATH.as_posix()}:{number}: duplicate baseline row {key}")
        entries[key] = int(parts[3])
    return entries


def load_baseline(repo_root: Path) -> dict[Key, int]:
    entries = read_baseline(repo_root)
    if entries is None:
        raise AdapterError("config", f"missing {BASELINE_PATH.as_posix()}; generate it with --write-baseline")
    return entries


def write_baseline(repo_root: Path, functions: list[FunctionSpan], allow_growth: bool = False) -> int:
    """Record every over-cap function at its current length.

    Raising an existing allowance, or adding one, is refused unless allow_growth is set.
    """
    over = over_cap_by_key(functions)
    recorded = read_baseline(repo_root)
    if recorded is not None and not allow_growth:
        raised = [
            key
            for key, function in sorted(over.items())
            if key not in recorded or function.code_lines > recorded[key]
        ]
        if raised:
            names = ", ".join(f"{path}:{name}#{occurrence}" for path, name, occurrence in raised)
            raise AdapterError(
                "config",
                f"refusing to raise {len(raised)} baseline allowance(s): {names}; pass --allow-growth to record them",
            )
    rows = [f"{path}\t{name}\t{occurrence}\t{function.code_lines}" for (path, name, occurrence), function in sorted(over.items())]
    header = [
        f"# function-length baseline: functions at or over {FAIL_THRESHOLD} code lines.",
        "# columns: path, name, occurrence (0-based within the file), code_lines.",
        "# regenerate: python -m sc_lint.lint_function_length --write-baseline",
    ]
    (repo_root / BASELINE_PATH).write_text("\n".join([*header, *rows]) + "\n", encoding="utf-8")
    return len(rows)


def evaluate(functions: list[FunctionSpan], baseline: dict[Key, int]) -> Evaluation:
    over = over_cap_by_key(functions)
    violations = tuple(
        Violation(function, baseline.get(key))
        for key, function in sorted(over.items())
        if key not in baseline or function.code_lines > baseline[key]
    )
    advisories = tuple(f for f in functions if WARN_THRESHOLD <= f.code_lines < FAIL_THRESHOLD)
    stale = tuple(key for key in sorted(baseline) if key not in over or over[key].code_lines < baseline[key])
    return Evaluation(advisories, violations, stale)


def render(function: FunctionSpan) -> str:
    return f"{function.path}:{function.start_line}-{function.end_line}: {function.name} ({function.code_lines} code lines)"


def render_violation(violation: Violation) -> str:
    detail = "new, at or over the cap" if violation.reason == "new" else f"grew from {violation.recorded_lines}"
    return f"{render(violation.function)}: {detail}"


def render_stale(key: Key) -> str:
    path, name, occurrence = key
    return f"{path} {name}#{occurrence}"


def build_data(repo_root: Path, evaluation: Evaluation, elapsed_ms: int) -> dict:
    output_dir = reset_findings_dir(repo_root, TOOL_NAME)
    failed = bool(evaluation.violations)
    summary = (
        f"function-length gate failed: {len(evaluation.violations)} new or grown function(s)"
        if failed
        else "function-length gate passed"
    )
    data = {
        "tool": f"sc-lint-{TOOL_NAME}",
        "status": "fail" if failed else "pass",
        "summary": summary,
        "violations": [
            {**asdict(v.function), "reason": v.reason, "recorded_lines": v.recorded_lines}
            for v in evaluation.violations
        ],
        "advisories": [asdict(f) for f in evaluation.advisories],
        "stale_baseline": [render_stale(key) for key in evaluation.stale],
        "elapsed_ms": elapsed_ms,
    }
    lines = [summary]
    lines.extend(f"violation: {render_violation(v)}" for v in evaluation.violations)
    lines.extend(f"advisory: {render(f)}" for f in evaluation.advisories)
    lines.extend(f"stale: {render_stale(key)}" for key in evaluation.stale)
    write_json(output_dir / "summary.json", data)
    write_text(output_dir / "summary.txt", "\n".join(lines) + "\n")
    data["artifact_dir"] = relative_artifact_path(repo_root, output_dir)
    return data


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Gate Rust function length against the recorded baseline.")
    parser.add_argument("--root", help="Repo root to inspect.")
    parser.add_argument("--config", help="Accepted for Python-adapter compatibility; the baseline path is fixed.")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--write-baseline", action="store_true", help="Regenerate the baseline from the tree.")
    parser.add_argument(
        "--allow-growth",
        action="store_true",
        help="With --write-baseline, record raised or new allowances instead of refusing.",
    )
    return parser.parse_args(argv[1:])


def main(argv: list[str]) -> int:
    args = None
    try:
        args = parse_args(argv)
        repo_root = discover_repo_root(args.root)
        started = monotonic_now()
        functions = scan_repo(repo_root)
        if args.write_baseline:
            count = write_baseline(repo_root, functions, allow_growth=args.allow_growth)
            print(f"wrote {BASELINE_PATH.as_posix()} with {count} entries")
            return 0
        evaluation = evaluate(functions, load_baseline(repo_root))
        data = build_data(repo_root, evaluation, int((monotonic_now() - started) * 1000))
        if args.json:
            write_adapter_json(success_payload(summary=data["summary"], data=data))
        else:
            print(data["summary"])
            for violation in evaluation.violations:
                print(f"error: {render_violation(violation)}")
            for function in evaluation.advisories:
                print(f"advisory: {render(function)}")
            for key in evaluation.stale:
                print(f"stale: {render_stale(key)}")
        return 1 if evaluation.violations else 0
    except AdapterError as error:
        if args is not None and args.json:
            write_adapter_json(error_payload(error))
        else:
            print(error.message, file=sys.stderr)
        return 3 if error.kind == "config" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
