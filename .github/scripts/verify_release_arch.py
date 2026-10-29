"""Verify release binary architecture without executing cross-compiled code."""

import argparse
from pathlib import Path
import struct
import zipfile


def machine(data: bytes) -> tuple[str, int]:
    if data[:4] == b"\x7fELF":
        if data[5] != 1:
            raise ValueError("expected little-endian ELF")
        return "elf", struct.unpack_from("<H", data, 18)[0]
    if data[:2] == b"MZ":
        offset = struct.unpack_from("<I", data, 0x3C)[0]
        if data[offset : offset + 4] != b"PE\0\0":
            raise ValueError("invalid PE signature")
        return "pe", struct.unpack_from("<H", data, offset + 4)[0]
    if data[:4] == b"\xcf\xfa\xed\xfe":
        return "macho", struct.unpack_from("<I", data, 4)[0]
    raise ValueError("unrecognized executable format")


EXPECTED = {
    "x86_64-unknown-linux-gnu": ("elf", 62),
    "aarch64-unknown-linux-gnu": ("elf", 183),
    "x86_64-pc-windows-msvc": ("pe", 0x8664),
    "aarch64-pc-windows-msvc": ("pe", 0xAA64),
    "x86_64-apple-darwin": ("macho", 0x01000007),
    "aarch64-apple-darwin": ("macho", 0x0100000C),
}


def verify(path: Path, target: str, manifest: Path | None = None) -> None:
    if path.suffix == ".whl":
        with zipfile.ZipFile(path) as wheel:
            extensions = [name for name in wheel.namelist() if name.endswith((".so", ".pyd"))]
            if len(extensions) != 1:
                raise ValueError(f"{path}: expected one native extension, found {extensions}")
            entries = [(extensions[0], wheel.read(extensions[0]))]
    else:
        suffix = ".exe" if "windows" in target else ""
        if manifest is None:
            raise ValueError("manifest is required for binary directories")
        from release_artifacts import load_manifest, _release_binaries

        names = [entry["name"] for entry in _release_binaries(load_manifest(manifest))]
        entries = [(name, (path / (name + suffix)).read_bytes()) for name in names]
    for name, data in entries:
        actual = machine(data)
        if actual != EXPECTED[target]:
            raise ValueError(f"{name}: expected {EXPECTED[target]}, got {actual}")
        print(f"{name}: {target} ({actual[0]}, {actual[1]:#x})")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=EXPECTED)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    verify(args.path, args.target, args.manifest)
