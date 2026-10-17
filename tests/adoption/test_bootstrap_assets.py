"""Cross-platform tests for the single consumer bootstrap."""
from __future__ import annotations

import ast
import os
from pathlib import Path
import subprocess
import sys

import pytest
from conftest import ProductArtifacts
from install_helpers import FIXTURES, ROOT, copy_fixture, run, run_subprocess

ASSET = ROOT / "crates/sc-lint/assets/bootstrap.py"

# The usage grammar of the POSIX `.sc-lint/bootstrap` at cc1d496, which printed:
#   usage: .sc-lint/bootstrap <setup|lint|test|upgrade> --config sc-lint.toml [--check] [--dry-run];
#   .sc-lint/bootstrap lint [selector] [host|all|linux|macos|windows] --config sc-lint.toml;
#   .sc-lint/bootstrap exec [--config sc-lint.toml] -- <argv...>
# bootstrap.py must list the same operations and arguments under its own name (AC3).
CC1D496_USAGE_GRAMMAR = (
    "usage: .sc-lint/bootstrap.py <setup|lint|test|upgrade|exec> --config sc-lint.toml [--check] [--dry-run]",
    ".sc-lint/bootstrap.py lint [selector] [host|all|linux|macos|windows] --config sc-lint.toml",
    ".sc-lint/bootstrap.py exec [--config sc-lint.toml] -- <argv...>",
)
CC1D496_USAGE_TOKENS = ("setup", "lint", "test", "upgrade", "exec", "--config sc-lint.toml", "[--check]", "[--dry-run]",
    "[selector]", "[host|all|linux|macos|windows]", "-- <argv...>")


def test_bootstrap_assets_are_identical_and_use_only_stdlib() -> None:
    paths = [ASSET, ROOT / "packages/sc-lint-adoption/.sc-lint/bootstrap.py", ROOT / ".sc-lint/bootstrap.py"]
    assert all(path.read_bytes() == ASSET.read_bytes() for path in paths)
    for path in paths:
        assert sorted(item.name for item in path.parent.glob("bootstrap*")) == ["bootstrap.py"]
    source = ASSET.read_text(encoding="utf-8")
    ast.parse(source, feature_version=(3, 6))
    tree = ast.parse(source)
    modules = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            modules.update(alias.name.split(".")[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            modules.add(node.module.split(".")[0])
    assert modules <= sys.stdlib_module_names
    for path in (ROOT / "crates/sc-lint/assets/consumer-Justfile",
                 ROOT / "packages/sc-lint-adoption/.sc-lint/justfile",
                 ROOT / "packages/sc-lint-adoption/templates/Justfile.default",
                 ROOT / "packages/sc-lint-adoption/templates/test.just", ROOT / "Justfile"):
        text = path.read_text(encoding="utf-8")
        assert "pw" + "sh" not in text
        for line in text.splitlines():
            if line.startswith("set windows-shell"):
                assert line == 'set windows-shell := ["cmd.exe", "/c"]'


@pytest.mark.parametrize("arguments", [[], ["unknown"], ["setup"], ["setup", "--config"],
    ["setup", "extra", "--config", "a"], ["lint", "a", "b", "c", "--config", "a"],
    ["lint", "", "all", "--config", "a"], ["test", "a", "b", "--config", "a"],
    ["exec"], ["exec", "--config", "a", "sc-lint"], ["exec", "--"]])
def test_usage_errors_report_one_shared_grammar(arguments: list[str]) -> None:
    result = run_subprocess([sys.executable, str(ASSET), *arguments], text=True, capture_output=True, check=False)
    assert result.returncode == 2
    usage_lines = [line for line in result.stderr.splitlines() if line.startswith("usage: .sc-lint/bootstrap.py")]
    assert len(usage_lines) == 1, result.stderr
    assert usage_lines[0].split("; ") == list(CC1D496_USAGE_GRAMMAR)
    for token in CC1D496_USAGE_TOKENS:
        assert token in usage_lines[0], token
    assert ".sc-lint/bootstrap " not in usage_lines[0]


def test_old_python_fails_before_importing_the_bootstrap() -> None:
    probe = "import runpy,sys; sys.version_info=(3,10,0); runpy.run_path(sys.argv[1],run_name='__main__')"
    result = run_subprocess([sys.executable, "-c", probe, str(ASSET)], text=True, capture_output=True, check=False)
    assert result.returncode == 1
    assert "CLI.SC_LINT_PYTHON_UNAVAILABLE" in result.stderr


@pytest.mark.integration
@pytest.mark.parametrize("exit_code", [0, 1, 2, 3, 4, 7])
def test_just_passes_through_product_exit_codes(exit_code: int, product_artifacts: ProductArtifacts, tmp_path: Path) -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    config = consumer / "sc-lint.toml"
    config.write_text(config.read_text().replace('minimum_version = "0.5.0"', f'minimum_version = "{product_artifacts.version}"'))
    source = tmp_path / "fake.rs"
    source.write_text('fn main() { if std::env::args().any(|a| a == "lint") { std::process::exit(std::env::var("SC_LINT_TEST_EXIT").unwrap().parse().unwrap()); } }')
    binary = tmp_path / ("fake.exe" if os.name == "nt" else "fake")
    build = run_subprocess(["rustc", str(source), "-o", str(binary)], text=True, capture_output=True, check=False)
    assert build.returncode == 0, build.stderr
    environment = {**os.environ, "SC_LINT_BIN": str(binary), "SC_LINT_WHEEL_DIR": str(product_artifacts.wheel_dir), "SC_LINT_TEST_EXIT": str(exit_code)}
    result = run_subprocess(["just", "lint"], cwd=consumer, env=environment, text=True, capture_output=True, check=False, timeout=300)
    assert result.returncode == exit_code, result.stdout + result.stderr


@pytest.mark.integration
def test_just_reports_bootstrap_install_failure(product_artifacts: ProductArtifacts, tmp_path: Path) -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    config = consumer / "sc-lint.toml"
    config.write_text(config.read_text().replace('minimum_version = "0.5.0"', f'minimum_version = "{product_artifacts.version}"'))
    environment = {**os.environ, "SC_LINT_BIN": str(product_artifacts.binary), "SC_LINT_WHEEL_DIR": str(tmp_path)}
    result = run_subprocess(["just", "lint"], cwd=consumer, env=environment, text=True, capture_output=True, check=False, timeout=300)
    assert result.returncode == 4, result.stdout + result.stderr
    assert "CLI.SC_LINT_PYTHON_UNAVAILABLE" in result.stderr
