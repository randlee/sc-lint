#!/usr/bin/env python3
"""Create and push the GitHub prerelease tag for the current workspace version.

The workspace version is the prerelease version. If `prerelease/v<version>`
already exists locally or on `origin`, the script fails closed: the archive
workflow verifies the tag version against the workspace manifest, so a
different version needs a deliberate version bump on its own branch.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Callable, Sequence

TAG_PREFIX = "prerelease/v"
WORKSPACE_TOML = Path("Cargo.toml")

Git = Callable[[Sequence[str]], str]


def run_git(args: Sequence[str]) -> str:
    return subprocess.run(["git", *args], check=True, text=True, capture_output=True).stdout


def workspace_version(text: str) -> str:
    version = tomllib.loads(text).get("workspace", {}).get("package", {}).get("version")
    if not isinstance(version, str):
        raise SystemExit("workspace.package.version is missing from Cargo.toml")
    parts = version.split(".")
    if len(parts) != 3 or not all(part.isdigit() for part in parts):
        raise SystemExit(f"workspace version {version!r} is not a stable X.Y.Z version")
    return version


def tag_exists(tag: str, git: Git) -> bool:
    if git(["tag", "--list", tag]).strip():
        return True
    return bool(git(["ls-remote", "--tags", "origin", f"refs/tags/{tag}"]).strip())


def plan(cargo_toml: str, git: Git) -> str:
    """Return the tag to create, or exit when the version was already published."""
    tag = f"{TAG_PREFIX}{workspace_version(cargo_toml)}"
    if tag_exists(tag, git):
        raise SystemExit(f"{tag} already exists; refusing to republish")
    return tag


def main(argv: Sequence[str] | None = None, git: Git = run_git) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true", help="print the tag that would be created")
    args = parser.parse_args(argv)
    tag = plan(WORKSPACE_TOML.read_text(encoding="utf-8"), git)
    if args.dry_run:
        print(f"would create and push {tag}")
        return 0
    git(["tag", tag])
    git(["push", "origin", f"refs/tags/{tag}"])
    print(f"created and pushed {tag} from {git(['rev-parse', 'HEAD']).strip()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
