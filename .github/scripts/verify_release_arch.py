"""Verify release architecture and Windows binary CRT imports without execution."""

import argparse
from pathlib import Path
import struct
import zipfile
from release_platforms import PLATFORMS


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


def _pe_offset(data: bytes, sections: list[tuple[int, int, int]], headers: int,
               rva: int, size: int) -> tuple[int, int]:
    """Map an RVA to file-backed bytes, returning offset and containing limit."""
    if rva < headers and rva + size <= min(headers, len(data)):
        return rva, min(headers, len(data))
    for address, raw_size, raw_offset in sections:
        relative = rva - address
        if 0 <= relative and relative + size <= raw_size:
            offset = raw_offset + relative
            limit = min(raw_offset + raw_size, len(data))
            if offset + size <= limit:
                return offset, limit
    raise ValueError(f"PE import RVA {rva:#x} is outside file-backed data")


def _pe_imports(data: bytes) -> list[str]:
    # https://learn.microsoft.com/en-us/windows/win32/debug/pe-format
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[:2] != b"MZ" or data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("invalid PE signature")
    section_count = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    optional_end = optional + optional_size
    magic = struct.unpack_from("<H", data, optional)[0]
    if magic not in (0x10B, 0x20B):
        raise ValueError(f"unsupported PE optional header magic {magic:#x}")
    directories = optional + (96 if magic == 0x10B else 112)
    if directories > optional_end or optional_end > len(data):
        raise ValueError("truncated PE optional header")
    directory_count = struct.unpack_from("<I", data, directories - 4)[0]
    if directory_count < 2:
        return []
    if directories + 16 > optional_end:
        raise ValueError("truncated PE import directory entry")
    import_rva, import_size = struct.unpack_from("<II", data, directories + 8)
    if import_rva == 0 and import_size == 0:
        return []
    if not import_rva or import_size < 20:
        raise ValueError("invalid PE import directory")
    headers = struct.unpack_from("<I", data, optional + 60)[0]
    sections = []
    for index in range(section_count):
        section = optional_end + index * 40
        address, raw_size, raw_offset = struct.unpack_from("<III", data, section + 12)
        sections.append((address, raw_size, raw_offset))
    table, _ = _pe_offset(data, sections, headers, import_rva, import_size)
    names = []
    for relative in range(0, import_size - 19, 20):
        descriptor = struct.unpack_from("<IIIII", data, table + relative)
        if not any(descriptor):
            return names
        name_rva = descriptor[3]
        if not name_rva:
            raise ValueError("PE import descriptor has no DLL name")
        offset, limit = _pe_offset(data, sections, headers, name_rva, 1)
        end = data.find(b"\0", offset, limit)
        if end <= offset:
            raise ValueError("PE import DLL name is empty or unterminated")
        names.append(data[offset:end].decode("ascii"))
    raise ValueError("unterminated PE import directory")


def pe_imports(data: bytes) -> list[str]:
    """Read PE32/PE32+ DLL names; malformed import tables fail closed."""
    try:
        return _pe_imports(data)
    except (struct.error, UnicodeDecodeError) as error:
        raise ValueError(f"invalid PE import table: {error}") from error


EXPECTED = {target: metadata[2] for target, metadata in PLATFORMS.items()}


def verify(path: Path, target: str, manifest: Path | None = None) -> None:
    if target not in EXPECTED:
        print(f"No architecture verifier for {target}; skipping static check")
        return
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
        entries = [(name + suffix, (path / (name + suffix)).read_bytes()) for name in names]
    for name, data in entries:
        actual = machine(data)
        if actual != EXPECTED[target]:
            raise ValueError(f"{name}: expected {EXPECTED[target]}, got {actual}")
        if actual[0] == "pe" and path.suffix != ".whl":
            try:
                imports = pe_imports(data)
            except ValueError as error:
                raise ValueError(f"{name}: {error}") from error
            for dll in imports:
                lower = dll.lower()
                if lower.endswith(".dll") and lower.startswith(("vcruntime", "msvcp", "api-ms-win-crt-")):
                    raise ValueError(f"{name}: imports {dll}; Windows release binaries must statically link the CRT")
        print(f"{name}: {target} ({actual[0]}, {actual[1]:#x})")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    verify(args.path, args.target, args.manifest)
