"""Exit-code tests for winget_manifest_check.py (lint-c27u.1 63-char hash defect)."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "winget_manifest_check.py"
X64 = "sc-lint_0.8.6_x86_64-pc-windows-msvc.zip"
ARM = "sc-lint_0.8.6_aarch64-pc-windows-msvc.zip"
X64_SHA = "384cd9c7b6236d9db027bed3f6e7733880130f1fefd633a7c7fc9b6e8ce1b182"
ARM_SHA = "d2c0b97f479d023b33148d7310ceafc953ba9d1fd212271db947749188db6b2e"


def manifest(x64_sha: str = X64_SHA.upper(), arm_sha: str = ARM_SHA.upper()) -> str:
    base = "https://github.com/randlee/sc-lint/releases/download/v0.8.6/"
    return (
        "Installers:\n"
        f"- Architecture: x64\n  InstallerUrl: {base}{X64}\n  InstallerSha256: {x64_sha}\n"
        f"- Architecture: arm64\n  InstallerUrl: {base}{ARM}\n  InstallerSha256: {arm_sha}\n"
    )


def run(tmp_path: Path, manifest_text: str, checksums: str):
    m = tmp_path / "installer.yaml"
    c = tmp_path / "checksums.txt"
    m.write_text(manifest_text, encoding="utf-8")
    c.write_text(checksums, encoding="utf-8")
    return subprocess.run(
        [sys.executable, "-I", str(SCRIPT), str(m), str(c)],
        capture_output=True,
        text=True,
    )


GOOD_SUMS = f"{X64_SHA}  {X64}\n{ARM_SHA}  {ARM}\n"


def test_good_manifest_passes_case_insensitively(tmp_path):
    result = run(tmp_path, manifest(), GOOD_SUMS)
    assert result.returncode == 0, result.stdout + result.stderr


def test_63_char_hash_fails(tmp_path):
    result = run(tmp_path, manifest(x64_sha=X64_SHA.upper().replace("D6", "6", 1)), GOOD_SUMS)
    assert result.returncode == 1
    assert "not 64 hex chars (len=63)" in result.stdout


def test_mismatched_hash_fails(tmp_path):
    wrong = "0" * 64
    result = run(tmp_path, manifest(arm_sha=wrong), GOOD_SUMS)
    assert result.returncode == 1
    assert "does not match checksums.txt" in result.stdout


def test_missing_checksums_entry_fails(tmp_path):
    result = run(tmp_path, manifest(), f"{X64_SHA}  {X64}\n")
    assert result.returncode == 1
    assert "no checksums.txt entry" in result.stdout


def test_manifest_without_installers_fails(tmp_path):
    result = run(tmp_path, "PackageIdentifier: randlee.sc-lint\n", GOOD_SUMS)
    assert result.returncode != 0
