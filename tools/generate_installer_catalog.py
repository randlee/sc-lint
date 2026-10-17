#!/usr/bin/env python3
"""Refresh Rust-owned analyzer catalogs used by the adoption installer."""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "packages" / "sc-lint-adoption" / "analyzer-catalog"
CRATES = {
    "boundary": "sc-lint-boundary",
    "portability": "sc-lint-portability",
    "runtime": "sc-lint-runtime",
}


def emit(analyzer: str) -> dict:
    with tempfile.TemporaryDirectory() as temporary:
        destination = Path(temporary) / f"{analyzer}.json"
        environment = {**os.environ, "SC_LINT_INSTALLER_CATALOG_OUTPUT": str(destination)}
        command = [
            "cargo",
            "test",
            "-q",
            "-p",
            CRATES[analyzer],
            "tests::emit_installer_catalog",
            "--",
            "--ignored",
            "--exact",
        ]
        subprocess.run(command, cwd=ROOT, env=environment, check=True)
        return json.loads(destination.read_text())


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--analyzer", choices=sorted(CRATES))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    analyzers = [args.analyzer] if args.analyzer else sorted(CRATES)
    generated = {name: emit(name) for name in analyzers}
    mismatches = []
    for name, catalog in generated.items():
        path = OUTPUT / f"{name}.json"
        if args.check:
            if not path.exists() or json.loads(path.read_text()) != catalog:
                mismatches.append(path)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(catalog, indent=2, sort_keys=True) + "\n")
    if mismatches:
        print("stale installer catalogs: " + ", ".join(str(path) for path in mismatches))
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
