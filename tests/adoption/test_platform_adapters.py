"""Installer coverage for optional portability platform-adapter records."""
from __future__ import annotations

import json
import os
import runpy
from pathlib import Path
import stat
import subprocess
import tomllib
from typing import Any

import pytest
from install_helpers import FIXTURES
from install_helpers import KIT
from install_helpers import ROOT
from install_helpers import WORKED_EXAMPLE_TIMEOUT_SECONDS
from install_helpers import copy_fixture
from install_helpers import run
from install_helpers import run_subprocess

ADAPTERS = [{
    "unix_module": "crates/transport/src/unix_socket.rs",
    "windows_module": "crates/transport/src/named_pipe.rs",
    "reason": "Reviewed private transport adapter.",
}]


def installer_schema() -> dict[str, Any]:
    return runpy.run_path(str(KIT / "install.py"), run_name="sc_lint_adoption_install")["load_schema"]()


def cargo_target_directory() -> Path:
    metadata = run_subprocess(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
        timeout=WORKED_EXAMPLE_TIMEOUT_SECONDS,
    )
    if metadata.returncode != 0:
        pytest.fail(
            "could not locate the Cargo target directory: `cargo metadata` "
            f"exited {metadata.returncode}\n{metadata.stderr}"
        )
    try:
        target_directory = json.loads(metadata.stdout)["target_directory"]
    except (json.JSONDecodeError, KeyError, TypeError) as exc:
        pytest.fail(f"cargo metadata did not report a target_directory: {exc}")
    return Path(target_directory).resolve()


def input_with_adapters(consumer: Path, adapters: object) -> Path:
    data = json.loads((FIXTURES / "install.json").read_text())
    data["analyzers"]["portability"]["platform_adapters"] = adapters
    path = consumer.parent / "install.json"
    path.write_text(json.dumps(data))
    return path


def tree_snapshot(root: Path) -> dict[str, tuple[str, int, bytes | None]]:
    snapshot = {}
    for path in root.rglob("*"):
        mode = stat.S_IMODE(path.stat().st_mode)
        if path.is_dir():
            snapshot[path.relative_to(root).as_posix()] = ("directory", mode, None)
        elif path.is_file():
            snapshot[path.relative_to(root).as_posix()] = ("file", mode, path.read_bytes())
    return snapshot


def portability_binary() -> Path:
    target_directory = cargo_target_directory()
    build = run_subprocess(
        ["cargo", "build", "--quiet", "-p", "sc-lint-portability", "--bin", "sc-lint-portability"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
        timeout=WORKED_EXAMPLE_TIMEOUT_SECONDS,
    )
    if build.returncode != 0:
        pytest.fail(
            "could not build the sc-lint-portability analyzer "
            f"(cargo build exited {build.returncode})\n{build.stderr}"
        )
    suffix = ".exe" if os.name == "nt" else ""
    binary = target_directory / "debug" / f"sc-lint-portability{suffix}"
    if not binary.is_file():
        pytest.fail(
            "cargo build succeeded but the sc-lint-portability analyzer was "
            f"not found at its reported target path: {binary}"
        )
    return binary


def test_cargo_target_directory_respects_cargo_target_dir(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    expected = tmp_path / "custom-target"
    monkeypatch.setenv("CARGO_TARGET_DIR", str(expected))
    assert cargo_target_directory() == expected.resolve()


@pytest.mark.parametrize(
    ("result", "expected_message"),
    [
        (
            subprocess.CompletedProcess([], 1, "", "metadata failure"),
            "could not locate the Cargo target directory: `cargo metadata` exited 1\nmetadata failure",
        ),
        (
            subprocess.CompletedProcess([], 0, "{}", ""),
            "cargo metadata did not report a target_directory: 'target_directory'",
        ),
    ],
)
def test_cargo_target_directory_reports_clear_failures(
    monkeypatch: pytest.MonkeyPatch,
    result: subprocess.CompletedProcess[str],
    expected_message: str,
) -> None:
    monkeypatch.setitem(globals(), "run_subprocess", lambda *_args, **_kwargs: result)

    with pytest.raises(pytest.fail.Exception) as error:
        cargo_target_directory()

    assert str(error.value) == expected_message


@pytest.mark.parametrize(
    ("result", "expected_message"),
    [
        (
            subprocess.CompletedProcess([], 1, "", "build failure"),
            "could not build the sc-lint-portability analyzer (cargo build exited 1)\nbuild failure",
        ),
        (
            subprocess.CompletedProcess([], 0, "", ""),
            "cargo build succeeded but the sc-lint-portability analyzer was not found at its reported target path",
        ),
    ],
)
def test_portability_binary_reports_clear_failures(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    result: subprocess.CompletedProcess[str],
    expected_message: str,
) -> None:
    monkeypatch.setitem(globals(), "cargo_target_directory", lambda: tmp_path)
    monkeypatch.setitem(globals(), "run_subprocess", lambda *_args, **_kwargs: result)

    with pytest.raises(pytest.fail.Exception) as error:
        portability_binary()

    assert str(error.value).startswith(expected_message)


def test_issue372_installer_roundtrip() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = input_with_adapters(consumer, ADAPTERS)
    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 0, result.stderr
    rendered = (consumer / "sc-lint.toml").read_text()
    assert "platform_adapters = [{" in rendered
    parsed = tomllib.loads(rendered)["tool"]["sc-lint"]["analyzers"]["portability"]
    assert parsed["platform_adapters"] == ADAPTERS

    installed_snapshot = tree_snapshot(consumer)
    repeated = run("--input", str(input_path), str(consumer))
    assert repeated.returncode == 0, repeated.stderr
    assert tree_snapshot(consumer) == installed_snapshot

    config_path = consumer / "sc-lint.toml"
    config_path.write_text(rendered.replace("Reviewed private transport adapter.", "Locally edited reason."))
    drift_snapshot = tree_snapshot(consumer)
    drift = run("--dry-run", "--input", str(input_path), str(consumer))
    assert drift.returncode == 1, drift.stderr
    assert "sc-lint.toml" in drift.stdout
    assert tree_snapshot(consumer) == drift_snapshot


@pytest.mark.parametrize(
    ("adapters", "expected_error"),
    [
        ("wrong", "platform_adapters must be an array"),
        ({}, "platform_adapters must be an array"),
        ([1], "platform_adapters[0] must be an object"),
        ([{"windows_module": "b.rs", "reason": "reason"}], "missing required fields"),
        ([{"unix_module": "a.rs", "reason": "reason"}], "missing required fields"),
        ([{"unix_module": "a.rs", "windows_module": "b.rs"}], "missing required fields"),
        ([{"unix_module": 1, "windows_module": "b.rs", "reason": "reason"}], "unix_module must be a string"),
        ([{"unix_module": "a.rs", "windows_module": False, "reason": "reason"}], "windows_module must be a string"),
        ([{"unix_module": "a.rs", "windows_module": "b.rs", "reason": None}], "reason must be a string"),
        ([{"unix_module": "", "windows_module": "b.rs", "reason": "reason"}], "unix_module does not match"),
        ([{"unix_module": "a.rs", "windows_module": "", "reason": "reason"}], "windows_module does not match"),
        ([{"unix_module": "a.rs", "windows_module": "b.rs", "reason": ""}], "reason does not match"),
        ([{"unix_module": "a.rs", "windows_module": "b.rs", "reason": "reason", "extra": "x"}], "unknown fields"),
    ],
)
def test_issue372_installer_rejects_structural_adapter_errors(
    adapters: object, expected_error: str
) -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = input_with_adapters(consumer, adapters)
    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 2
    assert expected_error in result.stderr, result.stderr
    assert not (consumer / "sc-lint.toml").exists()


@pytest.mark.parametrize(
    ("unix_module", "expected_code"),
    [
        ("../outside.rs", "ADAPTER_PATH"),
        ("src/missing.rs", "ADAPTER_MISSING"),
    ],
)
def test_issue372_installer_renders_semantic_cases_for_rust_rejection(
    unix_module: str, expected_code: str
) -> None:
    consumer = copy_fixture("empty-workspace")
    adapters = [{
        **ADAPTERS[0],
        "unix_module": unix_module,
        "windows_module": "src/lib.rs",
    }]
    input_path = input_with_adapters(consumer, adapters)
    install = run("--input", str(input_path), str(consumer))
    assert install.returncode == 0, install.stderr
    rendered = (consumer / "sc-lint.toml").read_text()
    assert tomllib.loads(rendered)["tool"]["sc-lint"]["analyzers"]["portability"]["platform_adapters"] == adapters

    analyzer = run_subprocess(
        [
            str(portability_binary()),
            "analyze",
            "--root",
            str(consumer),
            "--config",
            str(consumer / "sc-lint.toml"),
            "--format",
            "json",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    assert analyzer.returncode == 3, analyzer.stderr
    assert expected_code in analyzer.stderr


@pytest.mark.parametrize("adapters", [
    {},
    [{"unix_module": "a.rs", "windows_module": "b.rs"}],
    [{"unix_module": "", "windows_module": "b.rs", "reason": "reason"}],
    [{"unix_module": "a.rs", "windows_module": "b.rs", "reason": "reason", "extra": "x"}],
])
def test_issue372_installer_rejects_legacy_structural_adapter_cases(adapters: object) -> None:
    consumer = copy_fixture("empty-workspace")
    result = run("--input", str(input_with_adapters(consumer, adapters)), str(consumer))
    assert result.returncode == 2
    assert not (consumer / ".sc-lint").exists()


def test_issue372_schema_fragment_only_extends_portability() -> None:
    schema = installer_schema()
    assert schema["properties"]["analyzers"]["properties"]["portability"]["properties"]["platform_adapters"]["type"] == "array"
