#!/usr/bin/env python3
"""Provision the source-maintainer venv (.sc-lint/venv) with this checkout's sc_lint wheel.

Self-contained on purpose: it runs with the host interpreter before `sc_lint`
is importable. Consumers never use this file; they get the wheel from PyPI via
`.sc-lint/bootstrap setup`.
"""
from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]
VENV_DIR = REPO_ROOT / ".sc-lint" / "venv"
WHEEL_DIR = REPO_ROOT / ".sc-lint" / "wheels"
PACKAGE_DIR = REPO_ROOT / "bindings" / "sc-lint-py"
STAMP = VENV_DIR / "sc-lint-source.stamp"


def venv_python() -> Path:
    if sys.platform == "win32":
        return VENV_DIR / "Scripts" / "python.exe"
    return VENV_DIR / "bin" / "python3"


def workspace_version() -> str:
    text = (REPO_ROOT / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^\[workspace\.package\][^\[]*?^version\s*=\s*"([^"]+)"', text, re.M | re.S)
    if match is None:
        raise SystemExit("source_venv: workspace version not found in Cargo.toml")
    return match.group(1)


def source_fingerprint() -> str:
    newest = 0.0
    for path in [PACKAGE_DIR / "Cargo.toml", PACKAGE_DIR / "pyproject.toml", *PACKAGE_DIR.rglob("*.rs"), *PACKAGE_DIR.rglob("*.py")]:
        newest = max(newest, path.stat().st_mtime)
    for path in (REPO_ROOT / "crates" / "sc-lint" / "src").rglob("*.rs"):
        newest = max(newest, path.stat().st_mtime)
    newest = max(newest, (REPO_ROOT / "Cargo.toml").stat().st_mtime)
    return f"{workspace_version()}:{newest:.0f}"


def main() -> int:
    python = venv_python()
    fingerprint = source_fingerprint()
    if python.is_file() and STAMP.is_file() and STAMP.read_text(encoding="utf-8").strip() == fingerprint:
        return 0
    if not python.is_file():
        subprocess.run([sys.executable, "-m", "venv", str(VENV_DIR)], check=True)
    version = workspace_version()
    wheel_dir = os.environ.get("SC_LINT_WHEEL_DIR")
    if not wheel_dir:
        # Build the wheel (and its dependencies) into .sc-lint/wheels so fixture
        # tests can provision consumer venvs offline via SC_LINT_WHEEL_DIR.
        WHEEL_DIR.mkdir(parents=True, exist_ok=True)
        for stale in WHEEL_DIR.glob("sc_lint-*.whl"):
            stale.unlink()
        print(f"source_venv: building sc_lint wheel into {WHEEL_DIR}", file=sys.stderr)
        subprocess.run(
            [str(python), "-m", "pip", "wheel", "--quiet", "--disable-pip-version-check", "-w", str(WHEEL_DIR), str(PACKAGE_DIR)],
            check=True,
        )
        wheel_dir = str(WHEEL_DIR)
    install = [str(python), "-m", "pip", "install", "--quiet", "--disable-pip-version-check", "--force-reinstall"]
    install += ["--no-index", "--find-links", wheel_dir, f"sc-lint=={version}"]
    print(f"source_venv: installing sc_lint into {VENV_DIR}", file=sys.stderr)
    subprocess.run(install, check=True)
    STAMP.write_text(fingerprint + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
