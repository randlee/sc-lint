"""Stage only manifest-declared release payloads and checksum those exact bytes."""
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import re
import shutil
import tomllib

from npm_release import filename as npm_filename
from release_manifest import release_archive_name
from release_python import wheel_asset_pattern, wheel_targets


def asset_patterns(manifest: dict, version: str, *, prerelease: bool = False) -> list[str]:
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("release version must be MAJOR.MINOR.PATCH")
    patterns = [
        re.escape(release_archive_name(manifest["project"], target, version))
        for target in manifest.get("release_targets", [])
    ]
    for distribution in manifest.get("python_distributions", []):
        for wheel in wheel_targets(distribution):
            if not wheel["platform"]:
                raise ValueError("release staging requires explicit manifest wheel platforms")
            patterns.append(wheel_asset_pattern(distribution["name"], wheel["platform"], version))
        if distribution.get("sdist") and not prerelease:
            name = re.escape(distribution["name"]).replace(r"\-", "[-_]")
            patterns.append(name + "-" + re.escape(version) + r"\.tar\.gz")
    if not prerelease:
        patterns.extend(re.escape(npm_filename(package["name"], version)) for package in manifest.get("npm_packages", []))
    if not patterns:
        raise ValueError("manifest declares no release assets")
    return patterns


def stage(manifest: dict, version: str, source: Path, destination: Path, *, prerelease: bool = False) -> None:
    files = sorted(path for path in source.rglob("*") if path.is_file())
    selected = {}
    for pattern in asset_patterns(manifest, version, prerelease=prerelease):
        matches = [path for path in files if re.fullmatch(pattern, path.name)]
        if len(matches) != 1:
            raise ValueError(f"expected exactly one asset matching {pattern!r}; found {len(matches)}")
        path = matches[0]
        if path.name in selected or path.is_symlink():
            raise ValueError(f"duplicate or symlink release asset: {path}")
        selected[path.name] = path
    # A fresh directory prevents stale files from being uploaded on a reused runner.
    destination.mkdir(parents=True, exist_ok=False)
    checksums = []
    for name, source_path in sorted(selected.items()):
        target = destination / name
        shutil.copyfile(source_path, target)
        checksums.append(f"{hashlib.sha256(target.read_bytes()).hexdigest()}  {name}\n")
    (destination / "checksums.txt").write_text("".join(checksums), encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--prerelease", action="store_true", help="stage the archive/wheel subset built by prereleases")
    args = parser.parse_args(argv)
    try:
        manifest = tomllib.loads(args.manifest.read_text(encoding="utf-8"))
        stage(manifest, args.version, args.source, args.destination, prerelease=args.prerelease)
    except (ValueError, OSError, KeyError) as error:
        parser.exit(1, f"release-stage: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
