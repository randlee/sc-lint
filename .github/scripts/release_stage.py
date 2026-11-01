"""Stage only manifest-declared release payloads and checksum those exact bytes."""
from __future__ import annotations

import argparse
import hashlib
import re
from pathlib import Path
import shutil
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
