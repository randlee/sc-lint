"""Architecture verification must inspect the payload, not just its filename."""

import struct
import subprocess
import sys
import tomllib
from pathlib import Path
from zipfile import ZipFile

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from verify_release_arch import machine, pe_imports, verify


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


def test_unknown_consumer_target_skips_static_check(tmp_path):
    root = Path(__file__).resolve().parents[3]
    result = subprocess.run(
        [sys.executable, str(root / ".github/scripts/verify_release_arch.py"),
         "--target", "x86_64-unknown-linux-musl", str(tmp_path / "not-built")],
        capture_output=True, text=True, timeout=30,
    )
    assert result.returncode == 0, result.stderr
    assert "skipping static check" in result.stdout


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


def pe_image(dlls=(), arch=0x8664, pe32=False):
    """Small PE with an import directory inside a section, not at its start."""
    data = bytearray(0x600)
    pe, optional = 0x80, 0x98
    optional_size = 0xE0 if pe32 else 0xF0
    directories = optional + (96 if pe32 else 112)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, pe)
    data[pe:pe + 4] = b"PE\0\0"
    struct.pack_into("<HH", data, pe + 4, arch, 1)
    struct.pack_into("<H", data, pe + 20, optional_size)
    struct.pack_into("<H", data, optional, 0x10B if pe32 else 0x20B)
    struct.pack_into("<I", data, optional + 60, 0x200)
    struct.pack_into("<I", data, directories - 4, 16)
    section = optional + optional_size
    data[section:section + 8] = b".fixture"
    struct.pack_into("<IIII", data, section + 8, 0x400, 0x3000, 0x400, 0x200)
    if dlls:
        struct.pack_into("<II", data, directories + 8, 0x3040, (len(dlls) + 1) * 20)
    name_offset = 0x400
    for index, dll in enumerate(dlls):
        encoded = dll.encode("ascii") + b"\0"
        struct.pack_into("<IIIII", data, 0x240 + index * 20,
                         0x3380, 0, 0, 0x3000 + name_offset - 0x200, 0x3380)
        data[name_offset:name_offset + len(encoded)] = encoded
        name_offset += len(encoded)
    # An unreferenced string must never be mistaken for an imported DLL.
    marker = b"VCRUNTIME140.dll\0"
    data[0x580:0x580 + len(marker)] = marker
    return bytes(data)


def binary_payload(tmp_path, data):
    root = Path(__file__).resolve().parents[3]
    manifest = root / "release/publish-artifacts.toml"
    names = [entry["name"] for entry in tomllib.loads(manifest.read_text())["release_binaries"]]
    for name in names:
        (tmp_path / f"{name}.exe").write_bytes(data)
    return manifest, names


@pytest.mark.parametrize(("target", "arch"), [
    ("x86_64-pc-windows-msvc", 0x8664), ("aarch64-pc-windows-msvc", 0xAA64),
])
@pytest.mark.parametrize("dll", ["VCRUNTIME140.dll", "vcruntime140_1.DLL", "MSVCP140.dll",
                                 "api-ms-win-crt-runtime-l1-1-0.dll"])
def test_windows_binaries_reject_dynamic_crt(tmp_path, target, arch, dll):
    manifest, names = binary_payload(tmp_path, pe_image(["KERNEL32.dll"], arch))
    failing = names[-1] + ".exe"
    (tmp_path / failing).write_bytes(pe_image(["KERNEL32.dll", dll], arch))
    with pytest.raises(ValueError) as error:
        verify(tmp_path, target, manifest)
    assert failing in str(error.value)
    assert dll in str(error.value)


@pytest.mark.parametrize(("target", "arch"), [
    ("x86_64-pc-windows-msvc", 0x8664), ("aarch64-pc-windows-msvc", 0xAA64),
])
@pytest.mark.parametrize("dlls", [[], ["KERNEL32.dll", "ntdll.dll", "api-ms-win-core-synch-l1-2-0.dll",
                                      "SHELL32.dll", "combase.dll", "USERENV.dll", "bcryptprimitives.dll"]])
def test_windows_binaries_accept_system_imports(tmp_path, target, arch, dlls):
    manifest, _ = binary_payload(tmp_path, pe_image(dlls, arch))
    verify(tmp_path, target, manifest)


@pytest.mark.parametrize("pe32", [False, True])
def test_pe_imports_decode_optional_header_variants(pe32):
    assert pe_imports(pe_image(["KERNEL32.dll", "MSVCP140.dll"], pe32=pe32)) == ["KERNEL32.dll", "MSVCP140.dll"]


def test_windows_wheel_keeps_dynamic_crt_policy(tmp_path):
    wheel = tmp_path / "fixture-win_amd64.whl"
    with ZipFile(wheel, "w") as archive:
        archive.writestr("fixture/_native.pyd", pe_image(["VCRUNTIME140.dll"]))
    verify(wheel, "x86_64-pc-windows-msvc")


@pytest.mark.parametrize("corruption", ["truncated_optional", "invalid_rva", "unterminated_table", "unterminated_name"])
def test_pe_imports_reject_malformed_tables(corruption):
    data = bytearray(pe_image(["KERNEL32.dll"]))
    if corruption == "truncated_optional":
        data = data[:0x100]
    elif corruption == "invalid_rva":
        struct.pack_into("<I", data, 0x98 + 112 + 8, 0x9000)
    elif corruption == "unterminated_table":
        struct.pack_into("<I", data, 0x98 + 112 + 12, 20)
    else:
        data[0x400:] = b"x" * (len(data) - 0x400)
    with pytest.raises(ValueError):
        pe_imports(data)
