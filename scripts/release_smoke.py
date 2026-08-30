#!/usr/bin/env python3
"""Release-archive smoke test (sprint G.3b, REQ-PRODUCT-020/021/024).

Extracts a staged release archive, installs it into a temporary copy of
``tests/fixtures/adoption/empty-workspace`` with ``sc-lint init --just``, and
runs the consumer kit recipes (``just setup``, ``just lint``, ``just test``,
plus the bootstrap ``upgrade --check --dry-run`` path) with
``SC_LINT_SOURCE_ROOT`` unset and no ``.just/`` directory present. Every step
must succeed using only the archive binaries and the ``sc_lint`` wheel.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE = REPO_ROOT / "tests" / "fixtures" / "adoption" / "empty-workspace"
WINDOWS = os.name == "nt"
BINARY_NAME = "sc-lint.exe" if WINDOWS else "sc-lint"
SOURCE_ONLY_ENV = ("SC_LINT_SOURCE_ROOT", "PYTHONPATH", "SC_LINT_RECORD")


def extract_archive(archive: Path, destination: Path) -> Path:
    destination.mkdir(parents=True, exist_ok=True)
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as zipped:
            zipped.extractall(destination)
    elif archive.suffixes[-2:] == [".tar", ".gz"]:
        with tarfile.open(archive, mode="r:gz") as tarred:
            tarred.extractall(destination, filter="data")
    else:
        raise SystemExit(f"unsupported release archive format: {archive}")
    binary = destination / BINARY_NAME
    if not binary.is_file():
        raise SystemExit(f"archive does not contain {BINARY_NAME}: {archive}")
    if not WINDOWS:
        binary.chmod(binary.stat().st_mode | 0o111)
        for sibling in destination.iterdir():
            if sibling.is_file() and sibling.name.startswith("sc-lint-") and sibling.suffix == "":
                sibling.chmod(sibling.stat().st_mode | 0o111)
    return binary


def consumer_env(archive_dir: Path, binary: Path, wheel_dir: Path | None) -> dict[str, str]:
    env = {key: value for key, value in os.environ.items() if key not in SOURCE_ONLY_ENV}
    env["PATH"] = os.pathsep.join([str(archive_dir), env.get("PATH", "")])
    env["SC_LINT_BIN"] = str(binary)
    if wheel_dir is not None:
        env["SC_LINT_WHEEL_DIR"] = str(wheel_dir)
    return env


def run(step: str, command: list[str], cwd: Path, env: dict[str, str]) -> subprocess.CompletedProcess[str]:
    print(f"release-smoke: {step}: {' '.join(command)}", flush=True)
    result = subprocess.run(command, cwd=cwd, env=env, text=True, capture_output=True)
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)
    if result.returncode != 0:
        raise SystemExit(f"release-smoke: {step} failed with exit code {result.returncode}")
    return result


def assert_self_contained(binary: Path, workspace: Path, env: dict[str, str]) -> None:
    result = run("version", [str(binary), "--json", "version"], workspace, env)
    payload = json.loads(result.stdout)
    data = payload.get("data", payload)
    if data.get("self_contained") is not True:
        raise SystemExit(f"release-smoke: archive binary is not self-contained: {result.stdout}")


def bootstrap_command(workspace: Path) -> list[str]:
    if WINDOWS:
        return ["pwsh", "-NoLogo", "-File", str(workspace / ".sc-lint" / "bootstrap.ps1")]
    return [str(workspace / ".sc-lint" / "bootstrap")]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path, help="staged release archive (.tar.gz or .zip)")
    parser.add_argument(
        "--wheel-dir",
        type=Path,
        default=None,
        help="offline wheel directory for SC_LINT_WHEEL_DIR (sc-lint wheel plus its dependencies)",
    )
    args = parser.parse_args()

    with tempfile.TemporaryDirectory(prefix="sc-lint-release-smoke-") as temp:
        temp_dir = Path(temp)
        archive_dir = temp_dir / "archive"
        workspace = temp_dir / "workspace"
        binary = extract_archive(args.archive.resolve(), archive_dir)
        shutil.copytree(FIXTURE, workspace)
        env = consumer_env(archive_dir, binary, args.wheel_dir.resolve() if args.wheel_dir else None)

        assert_self_contained(binary, workspace, env)
        run("init", [str(binary), "init", "--just"], workspace, env)
        if (workspace / ".just").exists():
            raise SystemExit("release-smoke: `sc-lint init --just` materialized a .just/ directory")

        for recipe in ("setup", "lint", "test"):
            run(recipe, ["just", recipe], workspace, env)
        run(
            "upgrade",
            [*bootstrap_command(workspace), "upgrade", "--config", "sc-lint.toml", "--check", "--dry-run"],
            workspace,
            env,
        )
        if (workspace / ".just").exists():
            raise SystemExit("release-smoke: a kit recipe created a .just/ directory")
    print("release-smoke: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
