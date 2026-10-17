"""Shared helpers for adoption-kit integration tests."""
from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
KIT = ROOT / "packages" / "sc-lint-adoption"
INSTALL = KIT / "install.py"
FIXTURES = ROOT / "tests" / "fixtures" / "adoption"
SUBPROCESS_TIMEOUT_SECONDS = 60
WORKED_EXAMPLE_TIMEOUT_SECONDS = 300


def copy_fixture(name: str) -> Path:
    temporary = Path(tempfile.mkdtemp())
    destination = temporary / "consumer"
    shutil.copytree(FIXTURES / name, destination)
    return destination


def run_subprocess(
    arguments: list[str],
    *,
    timeout: float = SUBPROCESS_TIMEOUT_SECONDS,
    **options: Any,
) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            arguments, stdin=subprocess.DEVNULL, timeout=timeout, **options
        )
    except subprocess.TimeoutExpired as error:
        raise AssertionError(
            f"subprocess timed out after {timeout}s: {arguments!r}\n"
            f"stdout: {error.stdout!r}\nstderr: {error.stderr!r}"
        ) from error


def run(
    *arguments: str,
    cwd: Path | None = None,
    timeout: float = SUBPROCESS_TIMEOUT_SECONDS,
) -> subprocess.CompletedProcess[str]:
    return run_subprocess(
        ["python3", str(INSTALL), *arguments],
        cwd=cwd,
        timeout=timeout,
        text=True,
        capture_output=True,
        check=False,
    )


def write_input(consumer: Path, **overrides: object) -> Path:
    data = json.loads((FIXTURES / "install.json").read_text())
    data.update(overrides)
    path = consumer.parent / "install.json"
    path.write_text(json.dumps(data))
    return path
