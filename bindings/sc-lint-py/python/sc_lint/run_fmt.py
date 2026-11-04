#!/usr/bin/env python3
from __future__ import annotations

from sc_lint.lint_common import discover_repo_root

import subprocess
import sys
from pathlib import Path


FMT_STEPS = {
    "write": ["just", "_fmt-write"],
    "apply": ["just", "_fmt-write"],
    "check": ["just", "_fmt-check"],
}


def resolve_command(mode: str) -> list[str] | None:
    return FMT_STEPS.get(mode)


USAGE = "usage: python -m sc_lint.run_fmt [check|write|apply]  (default: check)"


def main(argv: list[str]) -> int:
    mode = argv[1] if len(argv) > 1 else "check"
    if mode in ("-h", "--help"):
        print(USAGE)
        return 0
    repo_root = discover_repo_root()

    command = resolve_command(mode)
    if command is None:
        valid = ", ".join(FMT_STEPS.keys())
        print(f"unknown fmt mode: {mode}", file=sys.stderr)
        print(f"expected one of: {valid}", file=sys.stderr)
        return 2

    completed = subprocess.run(command, cwd=repo_root)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
