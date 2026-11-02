"""Stage only manifest-declared release payloads and checksum those exact bytes."""
from __future__ import annotations

import argparse
import hashlib
import re
from pathlib import Path
import shutil
import sys
import tomllib

from release_python import asset_rules


def stage(manifest: dict, version: str, source: Path, destination: Path, *, prerelease: bool = False) -> None:
    files = sorted(path for path in source.rglob("*") if path.is_file())
    selected = {}
    for pattern, count in asset_rules(manifest, version, prerelease=prerelease):
        matches = [path for path in files if re.fullmatch(pattern, path.name)]
        if len(matches) != count:
            raise ValueError(f"expected {count} asset(s) matching {pattern!r}; found {len(matches)}")
        for path in matches:
            if path.name in selected or path.is_symlink():
                raise ValueError(f"duplicate or symlink release asset: {path}")
            selected[path.name] = path
    if not selected:
        raise ValueError("manifest declares no release assets")
    # A fresh directory prevents stale files from being uploaded on a reused runner.
    destination.mkdir(parents=True, exist_ok=False)
    checksums = []
    for name, source_path in sorted(selected.items()):
        target = destination / name
        shutil.copyfile(source_path, target)
        checksums.append(f"{hashlib.sha256(target.read_bytes()).hexdigest()}  {name}\n")
    (destination / "checksums.txt").write_text("".join(checksums), encoding="utf-8")


def upload_list(directory: Path) -> list[str]:
    files = sorted(directory.iterdir())
    if any(not path.is_file() or path.is_symlink() for path in files):
        raise ValueError("upload directory must contain only staged regular files")
    names = [path.name for path in files]
    if "checksums.txt" not in names or len(names) < 2:
        raise ValueError("upload directory requires payloads and checksums.txt")
    return names


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if argv[:1] == ["upload-list"]:
        parser = argparse.ArgumentParser(description="List the exact staged files to upload.")
        parser.add_argument("command", choices=["upload-list"])
        parser.add_argument("directory", type=Path)
        args = parser.parse_args(argv)
        try:
            print("\n".join(upload_list(args.directory)))
        except (ValueError, OSError) as error:
            parser.exit(1, f"release-stage: {error}\n")
        return 0
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
