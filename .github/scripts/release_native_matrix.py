"""The native verification matrix: the GitHub-hosted runner and Python that verify each release target.

This is host policy, not a release-target inventory. The manifest owns the inventory of
targets, archives and wheels; this module decides where each one is verified.
"""
from __future__ import annotations

from verify_release_arch import EXPECTED

NATIVE_RUNNERS = {
    ("Linux", "x86_64"): "ubuntu-24.04",
    ("Linux", "aarch64"): "ubuntu-24.04-arm",
    ("Darwin", "x86_64"): "macos-15-intel",
    ("Darwin", "aarch64"): "macos-15",
    ("Windows", "x86_64"): "windows-latest",
    ("Windows", "aarch64"): "windows-11-arm",
}
# Wheels are verified on Python 3.9 (the abi3-py39 floor); Windows ARM64 needs 3.11+.
WHEEL_PYTHON = "3.9"
WIN_ARM64_TARGET = "aarch64-pc-windows-msvc"
WIN_ARM64_PYTHON = "3.11"


def native_host(target: str) -> tuple[str, str]:
    if target not in EXPECTED:
        raise ValueError(f"{target}: no supported native GitHub-hosted runner/architecture verifier")
    kind = EXPECTED[target][0]
    return {"elf": "Linux", "macho": "Darwin", "pe": "Windows"}[kind], target.split("-", 1)[0]


def wheel_specs(manifest: dict, target: str) -> list[tuple[str, str]]:
    return [
        (distribution["name"].replace("-", "_"), wheel["platform"])
        for distribution in manifest.get("python_distributions", [])
        for wheel in distribution.get("wheels", []) if wheel["target"] == target
    ]


def native_matrix(manifest: dict) -> dict:
    rows = []
    for entry in manifest.get("release_targets", []):
        target = entry["target"]
        host = native_host(target)
        runner = NATIVE_RUNNERS.get(host)
        if runner is None:
            raise ValueError(f"{target}: no native GitHub-hosted runner for {host}")
        if not wheel_specs(manifest, target):
            raise ValueError(f"{target}: manifest declares no wheel to verify")
        python_version = WIN_ARM64_PYTHON if target == WIN_ARM64_TARGET else WHEEL_PYTHON
        rows.append({
            "target": target, "runner": runner, "python_arch": "arm64" if host[1] == "aarch64" else "x64",
            "python_version": python_version,
        })
    if not rows:
        raise ValueError("release manifest declares no release targets")
    if len({row["target"] for row in rows}) != len(rows):
        raise ValueError("release manifest contains duplicate release targets")
    return {"include": rows}
