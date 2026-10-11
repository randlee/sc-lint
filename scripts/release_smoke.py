#!/usr/bin/env python3
"""Release-archive smoke test (sprint G.3b, REQ-PRODUCT-020/021/024).

Extracts a staged release archive, installs it into a temporary copy of
``tests/fixtures/adoption/empty-workspace`` with ``sc-lint init --just``, and
runs the documentation bundle integrity check plus the consumer kit recipes
(``just setup``, ``just lint``, ``just test``, plus the bootstrap
``upgrade --check --dry-run`` path) with
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
import tomllib
import zipfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE = REPO_ROOT / "tests" / "fixtures" / "adoption" / "empty-workspace"
WINDOWS = os.name == "nt"
SOURCE_ONLY_ENV = ("SC_LINT_SOURCE_ROOT", "PYTHONPATH", "SC_LINT_RECORD")


def published_layout(manifest_path: Path, version: str, target: str) -> tuple[str, str, Path]:
    """Return the kit archive root, primary binary name, and docs destination."""
    manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    project = manifest.get("project", {})
    archive_prefix = project.get("archive_prefix")
    if not isinstance(archive_prefix, str) or not archive_prefix:
        raise SystemExit("release-smoke: publish manifest has no project.archive_prefix")

    binaries = manifest.get("release_binaries")
    if not isinstance(binaries, list):
        raise SystemExit("release-smoke: publish manifest has no release_binaries")
    primary = next((binary for binary in binaries if binary.get("name") == "sc-lint"), None)
    if primary is None:
        raise SystemExit("release-smoke: publish manifest has no sc-lint binary")
    bundles = primary.get("bundled_paths", [])
    docs_bundle = next(
        (bundle for bundle in bundles if bundle.get("source") == "docs-bundle"), None
    )
    if docs_bundle is None or not isinstance(docs_bundle.get("destination"), str):
        raise SystemExit("release-smoke: publish manifest has no docs-bundle destination")

    return (
        f"{archive_prefix}_{version}_{target}",
        "sc-lint.exe" if "windows" in target else "sc-lint",
        Path(docs_bundle["destination"]),
    )


def extract_archive(
    archive: Path,
    destination: Path,
    *,
    manifest: Path,
    version: str,
    target: str,
) -> tuple[Path, Path, Path]:
    destination.mkdir(parents=True, exist_ok=True)
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as zipped:
            zipped.extractall(destination)
    elif archive.suffixes[-2:] == [".tar", ".gz"]:
        with tarfile.open(archive, mode="r:gz") as tarred:
            tarred.extractall(destination, filter="data")
    else:
        raise SystemExit(f"unsupported release archive format: {archive}")

    archive_root_name, binary_name, docs_destination = published_layout(manifest, version, target)
    package_root = destination / archive_root_name
    binary = package_root / "bin" / binary_name
    if not binary.is_file():
        raise SystemExit(
            f"archive does not contain the published kit binary at "
            f"{archive_root_name}/bin/{binary_name}: {archive}"
        )
    docs_bundle = package_root / docs_destination
    if not docs_bundle.is_dir():
        raise SystemExit(
            f"archive does not contain the manifest-declared docs bundle at "
            f"{archive_root_name}/{docs_destination}: {archive}"
        )
    if not WINDOWS:
        binary.chmod(binary.stat().st_mode | 0o111)
        for sibling in binary.parent.iterdir():
            if sibling.is_file() and sibling.name.startswith("sc-lint-") and sibling.suffix == "":
                sibling.chmod(sibling.stat().st_mode | 0o111)
    return binary, package_root, docs_bundle


def consumer_env(binary: Path, wheel_dir: Path | None) -> dict[str, str]:
    env = {key: value for key, value in os.environ.items() if key not in SOURCE_ONLY_ENV}
    env["PATH"] = os.pathsep.join([str(binary.parent), env.get("PATH", "")])
    env["SC_LINT_BIN"] = str(binary)
    if wheel_dir is not None:
        env["SC_LINT_WHEEL_DIR"] = str(wheel_dir)
    return env


def assert_published_docs(binary: Path, expected_bundle: Path, workspace: Path, env: dict[str, str]) -> None:
    result = run("docs", [str(binary), "--json", "docs", "--path"], workspace, env)
    payload = json.loads(result.stdout)
    data = payload.get("data", payload)
    actual_bundle = Path(data.get("bundle_path", ""))
    if actual_bundle.resolve() != expected_bundle.resolve():
        raise SystemExit(
            "release-smoke: docs command did not discover the manifest-declared bundle: "
            f"expected {expected_bundle}, got {actual_bundle}"
        )


def assert_documentation_integrity(docs_bundle: Path, publish_manifest: Path) -> None:
    command = [
        sys.executable,
        str(REPO_ROOT / "scripts" / "release_artifacts.py"),
        "validate-documentation-bundle",
        "--manifest",
        str(REPO_ROOT / "release" / "sc-lint-release-checks.toml"),
        "--publish-manifest",
        str(publish_manifest.resolve()),
        "--documentation-directory",
        str(docs_bundle.resolve()),
    ]
    print(f"release-smoke: docs-integrity: {' '.join(command)}", flush=True)
    result = subprocess.run(command, cwd=REPO_ROOT, text=True, capture_output=True)
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()
        raise SystemExit(
            f"release-smoke: docs-integrity failed with exit code {result.returncode}: {detail}"
        )
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)


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
    parser.add_argument("--manifest", required=True, type=Path, help="rendered publish-artifacts.toml")
    parser.add_argument("--version", required=True, help="release version used in the archive root")
    parser.add_argument("--target", required=True, help="release target used in the archive root")
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
        binary, _package_root, docs_bundle = extract_archive(
            args.archive.resolve(),
            archive_dir,
            manifest=args.manifest.resolve(),
            version=args.version,
            target=args.target,
        )
        assert_documentation_integrity(docs_bundle, args.manifest)
        shutil.copytree(FIXTURE, workspace)
        env = consumer_env(binary, args.wheel_dir.resolve() if args.wheel_dir else None)

        assert_self_contained(binary, workspace, env)
        assert_published_docs(binary, docs_bundle, workspace, env)
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
