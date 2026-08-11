#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

from lint_common import discover_repo_root
from lint_common import workspace_crate_section_lines


DEPRECATED_CONFIG_LINES = (
    "vulnerability = ",
    "unlicensed = ",
)


def build_command(config_path: Path, version: tuple[int, int]) -> list[str]:
    checks = ["advisories", "bans", "licenses", "sources"]
    if version >= (0, 20):
        return ["cargo-deny", "--config", str(config_path), "check", *checks]
    return ["cargo-deny", "check", "--config", str(config_path), *checks]


def detect_version() -> tuple[int, int]:
    completed = subprocess.run(
        ["cargo-deny", "--version"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise RuntimeError(f"could not determine cargo-deny version: {detail}")

    match = re.search(r"cargo-deny\s+(\d+)\.(\d+)", completed.stdout)
    if match is None:
        raise RuntimeError(f"unrecognized cargo-deny version output: {completed.stdout.strip()}")
    return int(match.group(1)), int(match.group(2))


def build_runtime_config(repo_root: Path) -> Path:
    source_path = repo_root / "deny.toml"
    text = source_path.read_text(encoding="utf-8")
    filtered_lines = [
        line
        for line in text.splitlines()
        if not any(line.lstrip().startswith(prefix) for prefix in DEPRECATED_CONFIG_LINES)
    ]
    temp_dir = Path(tempfile.mkdtemp(prefix="atm-lint-deny-"))
    runtime_path = temp_dir / "deny.toml"
    runtime_path.write_text("\n".join(filtered_lines).rstrip() + "\n", encoding="utf-8")
    return runtime_path


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Run cargo-deny with the repo policy.")
    parser.add_argument("--root", help="Repo root to inspect.")
    args = parser.parse_args(argv[1:])
    repo_root = discover_repo_root(args.root)

    if shutil.which("cargo-deny") is None:
        print("cargo-deny is not installed; install it to run this lint", file=sys.stderr)
        return 2

    for line in workspace_crate_section_lines(repo_root):
        print(line)

    runtime_config = build_runtime_config(repo_root)
    try:
        version = detect_version()
    except RuntimeError as error:
        print(str(error), file=sys.stderr)
        return 2
    completed = subprocess.run(
        build_command(runtime_config, version),
        cwd=repo_root,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if completed.stdout:
        print(completed.stdout, end="")
    if completed.stderr:
        print(completed.stderr, end="", file=sys.stderr)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
