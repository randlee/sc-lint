#!/usr/bin/env python3
"""Run every command even after one fails, so one run reports every failure."""
from __future__ import annotations

import subprocess
import sys


def main(argv: list[str]) -> int:
    failed: list[str] = []
    for command in argv[1:]:
        print(f"run_all: {command}", flush=True)
        if subprocess.run(command, shell=True, check=False).returncode != 0:
            failed.append(command)
    for command in failed:
        print(f"run_all: FAILED {command}", file=sys.stderr)
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
