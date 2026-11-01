"""Drive the shared release build recipe with controlled command inputs."""

import os
import json
from pathlib import Path
import subprocess

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / ".github/scripts/build_release_target.sh"


@pytest.mark.parametrize(
    ("kind", "target", "runner", "manylinux", "expected"),
    [
        ("binaries", "aarch64-unknown-linux-gnu", "ubuntu-latest", "", "cargo zigbuild --release --target aarch64-unknown-linux-gnu.2.28 --bin fixture"),
        ("binaries", "x86_64-pc-windows-msvc", "windows-latest", "", "cargo build --release --target x86_64-pc-windows-msvc --bin fixture"),
        ("wheel", "aarch64-pc-windows-msvc", "windows-latest", "off", "maturin build --release --locked --manifest-path binding/Cargo.toml --out dist --target aarch64-pc-windows-msvc --features pyo3/generate-import-lib"),
        ("wheel", "aarch64-unknown-linux-gnu", "ubuntu-latest", "2_28", "maturin build --release --locked --manifest-path binding/Cargo.toml --out dist --target aarch64-unknown-linux-gnu --compatibility manylinux_2_28 --zig"),
        ("wheel", "", "ubuntu-24.04-arm", "", "maturin build --release --manifest-path binding/Cargo.toml --out dist --compatibility manylinux2014 --zig"),
        ("wheel", "", "macos-latest", "", "maturin build --release --manifest-path binding/Cargo.toml --out dist"),
        ("wheel", "", "windows-latest", "", "maturin build --release --manifest-path binding/Cargo.toml --out dist"),
    ],
)
def test_shared_recipe_commands(tmp_path, kind, target, runner, manylinux, expected):
    bindir = tmp_path / "bin"
    bindir.mkdir()
    log = tmp_path / "commands"
    for name in ("python3", "python", "cargo", "maturin", "rustup"):
        command = bindir / name
        command.write_text(
            '#!/bin/sh\n'
            'printf "%s %s cross=%s version=%s deployment_set=%s\\n" "$(basename "$0")" "$*" "${PYO3_CROSS-}" "${PYO3_CROSS_PYTHON_VERSION-}" "${MACOSX_DEPLOYMENT_TARGET+x}" >> "$CALL_LOG"\n'
            'if [ "$1" = .github/scripts/release_artifacts.py ]; then printf "%s\\n" "--bin fixture"; fi\n'
            'if [ "$1" = -m ] && [ "$2" = venv ]; then mkdir -p "$3/bin"; cp "$0" "$3/bin/python"; fi\n'
        )
        command.chmod(0o755)
    env = {**os.environ, "PATH": f"{bindir}{os.pathsep}{os.environ['PATH']}", "CALL_LOG": str(log),
           "BUILD_TARGET": target, "BUILD_RUNNER": runner, "BUILD_MANYLINUX": manylinux,
           "BUILD_SYSTEM": "maturin", "BUILD_SOURCE": "binding", "BUILD_CARGO_MANIFEST": "binding/Cargo.toml",
           "RELEASE_ARTIFACT_MANIFEST": "release/publish-artifacts.toml", "RUNNER_TEMP": str(tmp_path),
           "MACOSX_DEPLOYMENT_TARGET": ""}
    env.pop("PYO3_CROSS", None)
    env.pop("PYO3_CROSS_PYTHON_VERSION", None)
    result = subprocess.run(["bash", str(SCRIPT), kind], cwd=ROOT, env=env, capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, result.stderr
    lines = log.read_text().splitlines()
    assert any(line.startswith(expected + " cross=") for line in lines)
    if kind == "wheel":
        assert any(line.endswith("deployment_set=") for line in lines if line.startswith("maturin "))
    if target == "aarch64-pc-windows-msvc" and kind == "wheel":
        assert any("cross=1 version=3.11 " in line for line in lines if line.startswith("maturin "))
    if kind == "binaries" and target == "aarch64-unknown-linux-gnu":
        assert any("-m venv " in line for line in lines)
        assert any("pip install cargo-zigbuild==0.20.1 ziglang==0.12.1" in line for line in lines)


@pytest.mark.parametrize(("system", "value", "expected"), [
    ("maturin", "", "unset"), ("setuptools", "", "unset"), ("maturin", "11.0", "11.0"),
])
def test_wheel_deployment_target_is_unset_only_when_empty(tmp_path, system, value, expected):
    bindir = tmp_path / "bin"
    bindir.mkdir()
    log = tmp_path / "deployment"
    for name in ("python", "maturin"):
        stub = bindir / name
        stub.write_text('#!/bin/sh\nprintf "%s\\n" "${MACOSX_DEPLOYMENT_TARGET-unset}" >> "$CALL_LOG"\n')
        stub.chmod(0o755)
    env = {**os.environ, "PATH": f"{bindir}{os.pathsep}{os.environ['PATH']}", "CALL_LOG": str(log),
           "BUILD_TARGET": "", "BUILD_RUNNER": "macos-latest", "BUILD_SYSTEM": system,
           "BUILD_SOURCE": "binding", "BUILD_CARGO_MANIFEST": "binding/Cargo.toml",
           "MACOSX_DEPLOYMENT_TARGET": value}
    result = subprocess.run(["bash", str(SCRIPT), "wheel"], cwd=ROOT, env=env,
                            capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, result.stderr
    assert log.read_text().splitlines() == ([expected, expected] if system == "setuptools" else [expected])


def test_both_workflows_use_the_same_build_recipe():
    for name, wheel_job in (("release.yml", "build-python-wheels"), ("prerelease-archive.yml", "build-wheels")):
        workflow = yaml.safe_load((ROOT / ".github/workflows" / name).read_text())
        binaries = next(step for step in workflow["jobs"]["build"]["steps"] if step.get("name") == "Build release binaries")
        wheels = next(step for step in workflow["jobs"][wheel_job]["steps"] if step.get("name") == "Build declared wheel")
        assert "bash .github/scripts/build_release_target.sh binaries" in binaries["run"]
        assert wheels["run"] == "bash .github/scripts/build_release_target.sh wheel"


@pytest.mark.parametrize("arm", [False, True])
def test_winget_patterns_are_selected_by_target_not_list_position(tmp_path, arm):
    workflow = yaml.safe_load((ROOT / ".github/workflows/winget-publish.yml").read_text())
    step = next(step for step in workflow["jobs"]["publish-winget"]["steps"] if step.get("id") == "installer_patterns")
    x64 = "x86_64-pc-windows-msvc"
    arm64 = "aarch64-pc-windows-msvc"
    x64_pattern = r"^fixture_.*_x86_64\-pc\-windows\-msvc\.zip$"
    arm_pattern = r"^fixture_.*_aarch64\-pc\-windows\-msvc\.zip$"
    channel = {"installer_target": x64}
    if arm:
        channel["installer_arm64_target"] = arm64
    config = {"project": {"archive_prefix": "fixture"}, "channel": channel,
              "release_targets": {x64: {"archive": "zip"}, arm64: {"archive": "zip"}},
              "asset_patterns": [arm_pattern, "renderer", x64_pattern]}
    output = tmp_path / "output"
    env = {**os.environ, "CHANNEL_CONFIG": json.dumps(config), "GITHUB_OUTPUT": str(output)}
    result = subprocess.run(["bash", "-c", step["run"]], cwd=ROOT, env=env, capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, result.stderr
    expected = f"regex=({x64_pattern})" + (f"|({arm_pattern})" if arm else "") + "\n"
    assert output.read_text() == expected
