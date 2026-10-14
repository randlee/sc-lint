"""Installer coverage for optional portability platform-adapter records."""
from __future__ import annotations

import json
import os
import runpy
from pathlib import Path
import stat
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
    build = run_subprocess(
        ["cargo", "build", "--quiet", "-p", "sc-lint-portability", "--bin", "sc-lint-portability"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
        timeout=WORKED_EXAMPLE_TIMEOUT_SECONDS,
    )
    assert build.returncode == 0, build.stderr
    suffix = ".exe" if os.name == "nt" else ""
    binary = ROOT / "target" / "debug" / f"sc-lint-portability{suffix}"
    assert binary.is_file(), f"built portability analyzer is missing: {binary}"
    return binary


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
