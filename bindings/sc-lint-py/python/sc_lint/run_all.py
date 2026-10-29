#!/usr/bin/env python3
"""Run every command even after one fails, so one run reports every failure."""
from __future__ import annotations

import subprocess
import sys


def main(argv: list[str]) -> int:
    failed: list[str] = []
    for command in argv[1:]:
        print(f"run_all: {command}", flush=True)
        result = subprocess.run(command, shell=True, check=False, capture_output=True, text=True)
        if result.returncode != 0:
            # The consumer runner reports stderr as the failed step's cause;
            # include stdout there too so cargo's named test failures survive.
            sys.stderr.write(result.stdout)
            sys.stderr.write(result.stderr)
            failed.append(command)
        else:
            sys.stdout.write(result.stdout)
            sys.stderr.write(result.stderr)
    for command in failed:
        print(f"run_all: FAILED {command}", file=sys.stderr)
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
