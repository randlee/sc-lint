"""Optional consumer-declared documentation bundle for release archives.

A manifest may declare ``[documentation_bundle]`` with ``source_directory``,
``package_directory`` and ``required_files``. When declared, the bundle is
validated, staged into every release archive and archive contents are checked.
Without the table nothing here has any effect.
"""
from __future__ import annotations

import argparse
import shutil
import tarfile
import zipfile
from pathlib import Path

from release_manifest import _require_keys, load_manifest


def documentation_bundle(manifest: dict) -> dict | None:
    """Return the validated optional [documentation_bundle] table, or None."""
    bundle = manifest.get("documentation_bundle")
    if bundle is None:
        return None
    if not isinstance(bundle, dict):
        raise SystemExit("[documentation_bundle] must be a table")
    _require_keys(
        bundle,
        ("source_directory", "package_directory", "required_files"),
        "[documentation_bundle]",
    )
    for key in ("source_directory", "package_directory"):
        _require_safe_relative_path(bundle[key], f"[documentation_bundle].{key}")
    files = bundle["required_files"]
    if not isinstance(files, list) or not files:
        raise SystemExit("[documentation_bundle].required_files must be a non-empty list")
    for entry in files:
        _require_safe_relative_path(entry, "[documentation_bundle].required_files entry")
    if len(set(files)) != len(files):
        raise SystemExit("[documentation_bundle].required_files must not contain duplicates")
    return bundle


def _require_safe_relative_path(value: object, label: str) -> None:
    if not isinstance(value, str) or not value:
        raise SystemExit(f"{label} must be a non-empty string")
    path = Path(value)
    if path.is_absolute() or ".." in path.parts or "\\" in value:
        raise SystemExit(f"{label} must be a relative path inside the repository: {value}")


def _bundle_files(bundle: dict) -> set[str]:
    return set(bundle["required_files"])


def validate_documentation_bundle(manifest: dict, source_directory: Path) -> None:
    """Require the source directory to hold exactly the declared files."""
    bundle = documentation_bundle(manifest)
    if bundle is None:
        return
    if not source_directory.is_dir():
        raise SystemExit(f"documentation bundle source is not a directory: {source_directory}")
    expected = _bundle_files(bundle)
    actual = {
        item.relative_to(source_directory).as_posix()
        for item in source_directory.rglob("*")
        if item.is_file()
    }
    missing = sorted(expected - actual)
    unexpected = sorted(actual - expected)
    if missing or unexpected:
        raise SystemExit(
            "documentation bundle does not match [documentation_bundle].required_files: "
            f"missing={missing} unexpected={unexpected}"
        )


def stage_documentation_bundle(manifest: dict, stage_directory: Path) -> None:
    """Validate the bundle source and copy it to <stage>/<package_directory>."""
    bundle = documentation_bundle(manifest)
    if bundle is None:
        return
    source = Path(bundle["source_directory"])
    validate_documentation_bundle(manifest, source)
    destination = stage_directory / bundle["package_directory"]
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(source, destination)


def validate_release_archive(
    manifest: dict, binaries: list[dict], archive: Path, *, windows: bool = False
) -> None:
    """Check an archive holds every release binary and, when declared, exactly the bundle."""
    members = _archive_members(archive)
    roots = {member.split("/", 1)[0] for member in members}
    if len(roots) != 1:
        raise SystemExit(f"archive must contain a single top-level directory, found {sorted(roots)}")
    root = roots.pop()
    relative = {member.split("/", 1)[1] for member in members if "/" in member}
    suffix = ".exe" if windows else ""
    missing = [
        f"bin/{binary['name']}{suffix}"
        for binary in binaries
        if f"bin/{binary['name']}{suffix}" not in relative
    ]
    if missing:
        raise SystemExit(f"{archive}: missing release binaries: {missing}")
    bundle = documentation_bundle(manifest)
    if bundle is None:
        return
    prefix = bundle["package_directory"].rstrip("/") + "/"
    present = {name[len(prefix):] for name in relative if name.startswith(prefix)}
    expected = _bundle_files(bundle)
    if present != expected:
        raise SystemExit(
            f"{archive}: documentation bundle in {root}/{prefix} does not match "
            f"[documentation_bundle].required_files: missing={sorted(expected - present)} "
            f"unexpected={sorted(present - expected)}"
        )


def _archive_members(archive: Path) -> list[str]:
    """Return file member names (directories excluded) of a tar.gz or zip archive."""
    if archive.name.endswith(".zip"):
        with zipfile.ZipFile(archive) as opened:
            return [name for name in opened.namelist() if not name.endswith("/")]
    with tarfile.open(archive, "r:*") as opened:
        return [member.name for member in opened.getmembers() if member.isfile()]


def package_config_with_bundle(manifest: dict, config: dict) -> dict:
    """Add the validated bundle to a release-package-config payload when declared."""
    bundle = documentation_bundle(manifest)
    return config if bundle is None else {**config, "documentation_bundle": bundle}


def register_documentation_commands(sub, release_binaries) -> None:
    """Register the bundle subcommands on release_artifacts.py's subparsers."""

    def validate(args: argparse.Namespace) -> int:
        validate_documentation_bundle(
            load_manifest(Path(args.manifest)), Path(args.documentation_directory)
        )
        return 0

    def stage(args: argparse.Namespace) -> int:
        stage_documentation_bundle(load_manifest(Path(args.manifest)), Path(args.stage_directory))
        return 0

    def archive(args: argparse.Namespace) -> int:
        manifest = load_manifest(Path(args.manifest))
        validate_release_archive(
            manifest, release_binaries(manifest), Path(args.archive), windows=args.windows
        )
        return 0

    p = sub.add_parser("validate-documentation-bundle")
    p.add_argument("--manifest", required=True)
    p.add_argument("--documentation-directory", required=True)
    p.set_defaults(func=validate)

    p = sub.add_parser("stage-documentation-bundle")
    p.add_argument("--manifest", required=True)
    p.add_argument("--stage-directory", required=True)
    p.set_defaults(func=stage)

    p = sub.add_parser("validate-release-archive")
    p.add_argument("--manifest", required=True)
    p.add_argument("--archive", required=True)
    p.add_argument("--windows", action="store_true")
    p.set_defaults(func=archive)
