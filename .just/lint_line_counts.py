#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import sys

from lint_common import classify_rust_test_scope
from lint_common import discover_repo_root
from lint_common import is_code_line
from lint_common import load_lint_config
from lint_common import monotonic_now
from lint_common import workspace_manifest_paths
from python_adapter import AdapterError
from python_adapter import error_payload
from python_adapter import success_payload
from python_adapter import write_json as write_adapter_json
from view_common import relative_artifact_path
from view_common import reset_findings_dir
from view_common import write_json
from view_common import write_text


TOOL_NAME = "line-counts"


@dataclass(frozen=True)
class LineLimitConfig:
    max_total_lines: int | None = None
    max_production_lines: int | None = None
    max_scoped_code_lines: int | None = None
    exclusions: dict[str, str] | None = None


@dataclass(frozen=True)
class FileCounts:
    crate_name: str
    crate_root: str
    path: str
    total_lines: int
    production_lines: int
    test_lines: int

    @property
    def scoped_code_lines(self) -> int:
        return self.production_lines + self.test_lines


@dataclass(frozen=True)
class CrateTotals:
    crate_name: str
    total_lines: int
    production_lines: int
    test_lines: int

    @property
    def scoped_code_lines(self) -> int:
        return self.production_lines + self.test_lines


def classify_lines(lines: list[str]) -> tuple[int, int]:
    test_scope = classify_rust_test_scope(lines)
    production_lines = 0
    test_lines = 0
    for line, in_test_scope in zip(lines, test_scope, strict=True):
        if not is_code_line(line):
            continue
        if in_test_scope:
            test_lines += 1
        else:
            production_lines += 1
    return production_lines, test_lines


def load_config(repo_root: Path, config_path: str | None = None) -> LineLimitConfig:
    config = load_lint_config(repo_root, config_path).get("line_counts", {})
    if not isinstance(config, dict):
        raise AdapterError("config", "[line_counts] must be a TOML table")

    def parse_limit(name: str) -> int | None:
        value = config.get(name)
        if value in (None, 0):
            return None
        if not isinstance(value, int) or value < 0:
            raise AdapterError("config", f"[line_counts].{name} must be a non-negative integer")
        return value

    exclusions = config.get("exclusions", {})
    if not isinstance(exclusions, dict):
        raise AdapterError("config", "[line_counts.exclusions] must be a TOML table")

    return LineLimitConfig(
        max_total_lines=parse_limit("max_total_lines"),
        max_production_lines=parse_limit("max_production_lines"),
        max_scoped_code_lines=parse_limit("max_scoped_code_lines"),
        exclusions={str(path): str(reason) for path, reason in exclusions.items()},
    )


def collect_file_counts(repo_root: Path, config: LineLimitConfig) -> list[FileCounts]:
    results: list[FileCounts] = []
    for manifest_path in workspace_manifest_paths(repo_root):
        crate_root = manifest_path.parent
        src_root = crate_root / "src"
        if not src_root.exists():
            continue
        for path in sorted(src_root.rglob("*.rs")):
            rel = path.relative_to(repo_root)
            rel_posix = rel.as_posix()
            if config.exclusions and rel_posix in config.exclusions:
                continue

            lines = path.read_text(encoding="utf-8").splitlines()
            production_lines, test_lines = classify_lines(lines)
            results.append(
                FileCounts(
                    crate_name=crate_root.name,
                    crate_root=crate_root.relative_to(repo_root).as_posix(),
                    path=rel_posix,
                    total_lines=len(lines),
                    production_lines=production_lines,
                    test_lines=test_lines,
                )
            )
    return results


def crate_totals(counts: list[FileCounts]) -> list[CrateTotals]:
    totals: dict[str, CrateTotals] = {}
    for count in counts:
        previous = totals.get(
            count.crate_name,
            CrateTotals(
                crate_name=count.crate_name,
                total_lines=0,
                production_lines=0,
                test_lines=0,
            ),
        )
        totals[count.crate_name] = CrateTotals(
            crate_name=count.crate_name,
            total_lines=previous.total_lines + count.total_lines,
            production_lines=previous.production_lines + count.production_lines,
            test_lines=previous.test_lines + count.test_lines,
        )
    return [totals[name] for name in sorted(totals)]


def evaluate_limits(counts: list[FileCounts], config: LineLimitConfig) -> list[str]:
    failures: list[str] = []
    for item in counts:
        if config.max_total_lines is not None and item.total_lines > config.max_total_lines:
            failures.append(
                f"{item.path}: total={item.total_lines} exceeds limit {config.max_total_lines}"
            )
        if (
            config.max_production_lines is not None
            and item.production_lines > config.max_production_lines
        ):
            failures.append(
                f"{item.path}: prod={item.production_lines} exceeds limit {config.max_production_lines}"
            )
        if (
            config.max_scoped_code_lines is not None
            and item.scoped_code_lines > config.max_scoped_code_lines
        ):
            failures.append(
                f"{item.path}: prod+test={item.scoped_code_lines} exceeds limit {config.max_scoped_code_lines}"
            )
    return failures


def limit_summary(config: LineLimitConfig) -> str:
    parts: list[str] = []
    if config.max_total_lines is not None:
        parts.append(f"total<={config.max_total_lines}")
    if config.max_production_lines is not None:
        parts.append(f"prod<={config.max_production_lines}")
    if config.max_scoped_code_lines is not None:
        parts.append(f"prod+test<={config.max_scoped_code_lines}")
    return ", ".join(parts) if parts else "no active limits"


def build_data(repo_root: Path, counts: list[FileCounts], config: LineLimitConfig, failures: list[str], elapsed_ms: int) -> dict:
    output_dir = reset_findings_dir(repo_root, TOOL_NAME)
    crate_summary = [
        {
            "crate": total.crate_name,
            "total": total.total_lines,
            "prod": total.production_lines,
            "test": total.test_lines,
            "prod_test": total.scoped_code_lines,
        }
        for total in crate_totals(counts)
    ]
    data = {
        "tool": f"sc-lint-{TOOL_NAME}",
        "status": "pass" if not failures else "fail",
        "summary": (
            "source file size limits satisfied"
            if not failures
            else "source file size limits exceeded"
        ),
        "limits": {
            "summary": limit_summary(config),
            "max_total_lines": config.max_total_lines,
            "max_production_lines": config.max_production_lines,
            "max_scoped_code_lines": config.max_scoped_code_lines,
            "exclusions": config.exclusions or {},
        },
        "files": [
            {
                "crate": item.crate_name,
                "path": item.path,
                "total": item.total_lines,
                "prod": item.production_lines,
                "test": item.test_lines,
                "prod_test": item.scoped_code_lines,
            }
            for item in counts
        ],
        "crate_totals": crate_summary,
        "findings": failures,
        "elapsed_ms": elapsed_ms,
    }
    write_json(output_dir / "summary.json", data)
    lines = [data["summary"], f"active limits: {data['limits']['summary']}"]
    if failures:
        lines.extend(["findings:", *failures])
    else:
        lines.append("findings: none")
    write_text(output_dir / "summary.txt", "\n".join(lines) + "\n")
    data["artifact_dir"] = relative_artifact_path(repo_root, output_dir)
    return data


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check Rust file size limits by crate.")
    parser.add_argument("--root", help="Repo root to inspect.")
    parser.add_argument("--config", help="Repo config file override.")
    parser.add_argument("--json", action="store_true")
    return parser.parse_args(argv[1:])


def main(argv: list[str]) -> int:
    try:
        args = parse_args(argv)
        repo_root = discover_repo_root(args.root)
        config = load_config(repo_root, args.config)
        started = monotonic_now()
        counts = collect_file_counts(repo_root, config)
        failures = evaluate_limits(counts, config)
        elapsed_ms = int((monotonic_now() - started) * 1000)
        data = build_data(repo_root, counts, config, failures, elapsed_ms)
        if args.json:
            write_adapter_json(success_payload(summary=data["summary"], data=data))
            return 0 if not failures else 1

        print(f"line-counts {'passed' if not failures else 'failed'}")
        print(f"active limits: {data['limits']['summary']}")
        if failures:
            for failure in failures:
                print(failure)
        return 0 if not failures else 1
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
                f"failed to evaluate line counts: {error}",
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
