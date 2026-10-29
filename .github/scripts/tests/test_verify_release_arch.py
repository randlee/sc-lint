"""Architecture verification must inspect the payload, not just its filename."""

import struct
import subprocess
import sys
from pathlib import Path
from zipfile import ZipFile

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from verify_release_arch import machine, verify


@pytest.mark.parametrize("arch,expected", [(62, ("elf", 62)), (183, ("elf", 183))])
def test_elf_machine(arch, expected):
    data = bytearray(64)
    data[:4] = b"\x7fELF"
    data[5] = 1
    struct.pack_into("<H", data, 18, arch)
    assert machine(data) == expected


@pytest.mark.parametrize("arch", [0x8664, 0xAA64])
def test_pe_machine(arch):
    data = bytearray(128)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 64)
    data[64:68] = b"PE\0\0"
    struct.pack_into("<H", data, 68, arch)
    assert machine(data) == ("pe", arch)


def test_wheel_payload_architecture_is_checked(tmp_path):
    wheel = tmp_path / "package-1-cp39-abi3-win_arm64.whl"
    data = bytearray(128)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 64)
    data[64:68] = b"PE\0\0"
    struct.pack_into("<H", data, 68, 0x8664)
    with ZipFile(wheel, "w") as archive:
        archive.writestr("package/_native.pyd", data)
    with pytest.raises(ValueError, match="expected"):
        verify(wheel, "aarch64-pc-windows-msvc")


def test_preflight_rejects_missing_arm_archive(tmp_path):
    root = Path(__file__).resolve().parents[3]
    manifest = (root / "release/publish-artifacts.toml").read_text()
    missing = '[[release_targets]]\ntarget = "aarch64-pc-windows-msvc"\nos = "windows-latest"\narchive = "zip"\n'
    assert missing in manifest
    path = tmp_path / "manifest.toml"
    path.write_text(manifest.replace(missing, ""))
    script = root / ".github/scripts/check_sc_lint_platforms.py"
    result = subprocess.run([sys.executable, str(script), str(path)], cwd=root, capture_output=True, text=True, timeout=30)
    assert result.returncode != 0
    assert "six release targets" in result.stderr


def test_pypi_verifier_rejects_mislabeled_payload(tmp_path):
    root = Path(__file__).resolve().parents[3]
    manifest = root / "release/publish-artifacts.toml"
    wheel = tmp_path / "sc_lint-0.8.3-cp39-abi3-win_arm64.whl"
    data = bytearray(128)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 64)
    data[64:68] = b"PE\0\0"
    struct.pack_into("<H", data, 68, 0x8664)
    with ZipFile(wheel, "w") as archive:
        archive.writestr("sc_lint/_native.pyd", data)
    result = subprocess.run(
        [sys.executable, str(root / ".github/scripts/verify_sc_lint_wheels.py"), str(manifest), str(tmp_path)],
        cwd=root, capture_output=True, text=True, timeout=30,
    )
    assert result.returncode != 0
    assert "expected" in result.stderr


def test_pypi_verifier_accepts_all_six_matching_payloads(tmp_path):
    root = Path(__file__).resolve().parents[3]
    for platform, kind, arch in [
        ("manylinux_2_28_x86_64", "elf", 62),
        ("manylinux_2_28_aarch64", "elf", 183),
        ("macosx_11_0_x86_64", "macho", 0x01000007),
        ("macosx_11_0_arm64", "macho", 0x0100000C),
        ("win_amd64", "pe", 0x8664),
        ("win_arm64", "pe", 0xAA64),
    ]:
        data = bytearray(128)
        if kind == "elf":
            data[:4] = b"\x7fELF"
            data[5] = 1
            struct.pack_into("<H", data, 18, arch)
        elif kind == "macho":
            data[:4] = b"\xcf\xfa\xed\xfe"
            struct.pack_into("<I", data, 4, arch)
        else:
            data[:2] = b"MZ"
            struct.pack_into("<I", data, 0x3C, 64)
            data[64:68] = b"PE\0\0"
            struct.pack_into("<H", data, 68, arch)
        wheel = tmp_path / f"sc_lint-0.8.3-cp39-abi3-{platform}.whl"
        with ZipFile(wheel, "w") as archive:
            archive.writestr("sc_lint/_native.pyd" if kind == "pe" else "sc_lint/_native.so", data)
    result = subprocess.run(
        [sys.executable, str(root / ".github/scripts/verify_sc_lint_wheels.py"),
         str(root / "release/publish-artifacts.toml"), str(tmp_path)],
        cwd=root, capture_output=True, text=True, timeout=30,
    )
    assert result.returncode == 0, result.stderr
