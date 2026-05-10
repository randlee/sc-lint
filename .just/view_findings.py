#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from lint_common import discover_repo_root
from python_adapter import AdapterError
from python_adapter import error_payload
from python_adapter import success_payload
from python_adapter import write_json as write_adapter_json
from view_common import findings_root
from view_common import relative_artifact_path
from view_common import reset_view_dir
from view_common import write_json
from view_common import write_text


TOOL_NAME = "findings"


def read_json(path: Path) -> dict | None:
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def build_index(repo_root: Path) -> dict:
    source_root = findings_root(repo_root)
    output_dir = reset_view_dir(repo_root, TOOL_NAME)
    entries: list[dict] = []
    for tool_dir in sorted(source_root.glob("*")) if source_root.exists() else []:
        if not tool_dir.is_dir():
            continue
        summary_path = tool_dir / "summary.json"
        summary = read_json(summary_path)
        if summary is None:
            continue
        entries.append(
            {
                "tool": summary.get("tool", tool_dir.name),
                "status": summary.get("status", "unknown"),
                "summary": summary.get("summary", ""),
                "finding_count": len(summary.get("findings", [])),
                "artifact_dir": relative_artifact_path(repo_root, tool_dir),
                "summary_json": relative_artifact_path(repo_root, summary_path),
            }
        )

    data = {
        "tool": "sc-lint-view-findings",
        "status": "pass",
        "summary": f"collated {len(entries)} findings artifact set(s)",
        "views": entries,
        "artifact_dir": relative_artifact_path(repo_root, output_dir),
        "source_root": relative_artifact_path(repo_root, source_root),
    }
    write_json(output_dir / "index.json", data)
    text_lines = [data["summary"]]
    for entry in entries:
        text_lines.append(
            f"{entry['tool']}: {entry['status']} ({entry['finding_count']} findings) -> {entry['summary_json']}"
        )
    if not entries:
        text_lines.append("no findings artifacts available")
    write_text(output_dir / "index.txt", "\n".join(text_lines) + "\n")
    return data


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Collate findings artifacts into a stable view index.")
    parser.add_argument("--root", help="Repo root to inspect.")
    parser.add_argument("--json", action="store_true")
    return parser.parse_args(argv[1:])


def main(argv: list[str]) -> int:
    try:
        args = parse_args(argv)
        repo_root = discover_repo_root(args.root)
        data = build_index(repo_root)
        if args.json:
            write_adapter_json(success_payload(summary=data["summary"], data=data))
            return 0
        print(data["summary"])
        print(relative_artifact_path(repo_root, repo_root / data["artifact_dir"]))
        return 0
    except AdapterError as error:
        print(json.dumps(error_payload(error), indent=2, sort_keys=True))
        return 1
    except Exception as error:  # pragma: no cover - contract guardrail
        print(
            json.dumps(
                error_payload(
                    AdapterError(
                        "backend_protocol",
                        f"failed to build findings view: {error}",
                        details={"tool": "sc-lint-view-findings"},
                    )
                ),
                indent=2,
                sort_keys=True,
            )
        )
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
