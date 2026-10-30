"""Release target metadata shared by planning and artifact verification."""

PLATFORMS = {
    "x86_64-unknown-linux-gnu": ("manylinux_2_28_x86_64", "ubuntu-latest", ("elf", 62)),
    "aarch64-unknown-linux-gnu": ("manylinux_2_28_aarch64", "ubuntu-latest", ("elf", 183)),
    "x86_64-apple-darwin": ("macosx_11_0_x86_64", "macos-latest", ("macho", 0x01000007)),
    "aarch64-apple-darwin": ("macosx_11_0_arm64", "macos-latest", ("macho", 0x0100000C)),
    "x86_64-pc-windows-msvc": ("win_amd64", "windows-latest", ("pe", 0x8664)),
    "aarch64-pc-windows-msvc": ("win_arm64", "windows-latest", ("pe", 0xAA64)),
}


def wheel_platform(target: str, deployment_target: str = "11.0") -> str:
    platform = PLATFORMS[target][0]
    if target.endswith("apple-darwin"):
        return platform.replace("macosx_11_0_", f"macosx_{deployment_target.replace('.', '_')}_", 1)
    return platform
