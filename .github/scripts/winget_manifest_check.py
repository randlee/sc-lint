#!/usr/bin/env python3
"""Verify a winget installer manifest against a release checksums.txt.

Usage: winget_manifest_check.py <installer.yaml> <checksums.txt>
Every InstallerSha256 must be 64 hex chars and exactly equal (case-insensitive)
to the checksums.txt entry for the asset named by its InstallerUrl.
"""
import re
import sys


def main(manifest, checksums):
    sums = {}
    for line in open(checksums, encoding="utf-8"):
        parts = line.split()
        if len(parts) == 2:
            sums[parts[1].lstrip("*")] = parts[0].lower()
    text = open(manifest, encoding="utf-8").read()
    pairs = re.findall(r"InstallerUrl: (\S+)\s+InstallerSha256: (\S+)", text)
    if not pairs:
        sys.exit("no InstallerUrl/InstallerSha256 pairs found")
    bad = 0
    for url, sha in pairs:
        asset = url.rsplit("/", 1)[-1]
        if not re.fullmatch(r"[0-9A-Fa-f]{64}", sha):
            reason = f"not 64 hex chars (len={len(sha)})"
        elif asset not in sums:
            reason = "no checksums.txt entry for asset"
        elif sums[asset] != sha.lower():
            reason = f"does not match checksums.txt ({sums[asset]})"
        else:
            reason = None
        print("OK  " if reason is None else "FAIL", asset, sha, reason or "")
        bad += reason is not None
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(*sys.argv[1:])
