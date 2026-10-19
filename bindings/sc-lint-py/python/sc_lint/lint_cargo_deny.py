#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable

from sc_lint.lint_common import discover_repo_root
from sc_lint.lint_common import write_console_output
from sc_lint.lint_common import workspace_crate_section_lines


DEPRECATED_CONFIG_LINES = (
    "vulnerability = ",
    "unlicensed = ",
)
DNS_RETRY_ATTEMPTS = 3
DNS_RETRY_DELAY_SECONDS = 2
DNS_RESOLUTION_FAILURE_MARKER = "could not resolve hostname"


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


def is_dns_resolution_failure(completed: subprocess.CompletedProcess[str]) -> bool:
    output = "\n".join((completed.stdout or "", completed.stderr or "")).lower()
    return completed.returncode != 0 and DNS_RESOLUTION_FAILURE_MARKER in output


def run_cargo_deny(
    command: list[str],
    repo_root: Path,
    *,
    runner: Callable[..., subprocess.CompletedProcess[str]] | None = None,
    sleeper: Callable[[float], None] | None = None,
) -> subprocess.CompletedProcess[str]:
    run = runner or subprocess.run
    sleep = sleeper or time.sleep
    for attempt in range(1, DNS_RETRY_ATTEMPTS + 1):
        completed = run(
            command,
            cwd=repo_root,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        if not is_dns_resolution_failure(completed) or attempt == DNS_RETRY_ATTEMPTS:
            return completed

        write_console_output(
            sys.stderr,
            f"cargo-deny DNS failure (attempt {attempt}/{DNS_RETRY_ATTEMPTS}); retrying in {DNS_RETRY_DELAY_SECONDS}s\n",
        )
        sleep(DNS_RETRY_DELAY_SECONDS)

    raise AssertionError("cargo-deny retry loop must return")


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
        write_console_output(
            sys.stderr, "cargo-deny is not installed; install it to run this lint\n"
        )
        return 2

    for line in workspace_crate_section_lines(repo_root):
        write_console_output(sys.stdout, f"{line}\n")

    runtime_config = build_runtime_config(repo_root)
    try:
        version = detect_version()
    except RuntimeError as error:
        write_console_output(sys.stderr, f"{error}\n")
        return 2
    completed = run_cargo_deny(build_command(runtime_config, version), repo_root)
    if completed.stdout:
        write_console_output(sys.stdout, completed.stdout)
    if completed.stderr:
        write_console_output(sys.stderr, completed.stderr)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
