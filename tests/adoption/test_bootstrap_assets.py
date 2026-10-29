"""Cross-platform tests for the single consumer bootstrap."""
from __future__ import annotations

import ast
import importlib.util
import os
from pathlib import Path
import subprocess
import sys

import pytest
from conftest import ProductArtifacts
from install_helpers import FIXTURES, ROOT, copy_fixture, run, run_subprocess

ASSET = ROOT / "crates/sc-lint/assets/bootstrap.py"


@pytest.mark.parametrize(
    ("system", "machine", "expected"),
    [
        ("Linux", "x86_64", ("x86_64-unknown-linux-gnu", "tar.gz")),
        ("Linux", "aarch64", ("aarch64-unknown-linux-gnu", "tar.gz")),
        ("Windows", "AMD64", ("x86_64-pc-windows-msvc", "zip")),
        ("Windows", "ARM64", ("aarch64-pc-windows-msvc", "zip")),
    ],
)
def test_release_target_selects_controlled_host(monkeypatch, system, machine, expected):
    spec = importlib.util.spec_from_file_location("test_bootstrap_target", ASSET)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    monkeypatch.setattr(module.platform, "system", lambda: system)
    monkeypatch.setattr(module.platform, "machine", lambda: machine)
    assert module.release_target() == expected

# AC5 (ADR-012 d1): every just asset that ships a windows-shell sets it to exactly this line.
WINDOWS_SHELL_SETTING = 'set windows-shell := ["cmd.exe", "/c"]'
# The root-level assets ship the setting; the included `.sc-lint/justfile` and `test.just`
# inherit it from the consumer's root Justfile and must not set one of their own.
WINDOWS_SHELL_ASSETS = ("crates/sc-lint/assets/consumer-Justfile", "packages/sc-lint-adoption/templates/Justfile.default",
    "Justfile")

# The usage grammar of the POSIX `.sc-lint/bootstrap` at cc1d496, which printed:
#   usage: .sc-lint/bootstrap <setup|lint|test|upgrade> --config sc-lint.toml [--check] [--dry-run];
#   .sc-lint/bootstrap lint [selector] [host|all|linux|macos|windows] --config sc-lint.toml;
#   .sc-lint/bootstrap exec [--config sc-lint.toml] -- <argv...>
# bootstrap.py must list the same operations and arguments under its own name (AC3), plus the
# l-6 `--fail-fast` lint flag and the test selector grammar.
CC1D496_USAGE_GRAMMAR = (
    "usage: .sc-lint/bootstrap.py <setup|lint|test|upgrade|exec> --config sc-lint.toml [--check] [--dry-run]",
    ".sc-lint/bootstrap.py lint [selector] [host|all|linux|macos|windows] --config sc-lint.toml",
    ".sc-lint/bootstrap.py lint [--fail-fast] [selector] [host|all|linux|macos|windows] --config sc-lint.toml",
    ".sc-lint/bootstrap.py test [--fail-fast|--list] [selector [args...]] --config sc-lint.toml",
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
    for relative in ("crates/sc-lint/assets/consumer-Justfile", "packages/sc-lint-adoption/.sc-lint/justfile",
                     "packages/sc-lint-adoption/templates/Justfile.default",
                     "packages/sc-lint-adoption/templates/test.just", "Justfile"):
        text = (ROOT / relative).read_text(encoding="utf-8")
        assert "pw" + "sh" not in text, relative
        shell_lines = [line for line in text.splitlines() if line.startswith("set windows-shell")]
        if relative in WINDOWS_SHELL_ASSETS:
            assert shell_lines == [WINDOWS_SHELL_SETTING], relative
        else:
            assert shell_lines == [], relative


@pytest.mark.parametrize("arguments", [[], ["unknown"], ["setup"], ["setup", "--config"],
    ["setup", "extra", "--config", "a"], ["lint", "a", "b", "c", "--config", "a"],
    ["lint", "", "all", "--config", "a"], ["test", "--config", "a", "--list", "x"],
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


def load_bootstrap():
    spec = importlib.util.spec_from_file_location("sc_lint_bootstrap", ASSET)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_bootstrap_test_forwards_selector_and_args() -> None:
    bootstrap = load_bootstrap()
    parsed = bootstrap.parse(["test", "--config", "c", "--fail-fast", "unit", "--nocapture", "--config", "x"])
    assert parsed == ("test", "c", ["--fail-fast"], ["unit", "--nocapture", "--config", "x"], [])


@pytest.mark.parametrize("arguments", [["test", "--config", "c", "--list", "unit"],
    ["test", "--config", "c", "--list", "--fail-fast"], ["test", "--config", "c", "--fail-fast", "--list"]])
def test_bootstrap_test_list_conflicts(arguments: list[str]) -> None:
    bootstrap = load_bootstrap()
    with pytest.raises(SystemExit) as raised:
        bootstrap.parse(arguments)
    assert raised.value.code == 2


def test_bootstrap_lint_accepts_fail_fast() -> None:
    bootstrap = load_bootstrap()
    parsed = bootstrap.parse(["lint", "--fail-fast", "ci", "all", "--config", "c"])
    assert parsed == ("lint", "c", ["--fail-fast"], ["ci", "all"], [])
    for operation in ("setup", "upgrade"):
        with pytest.raises(SystemExit) as raised:
            bootstrap.parse([operation, "--config", "c", "--fail-fast"])
        assert raised.value.code == 2
    with pytest.raises(SystemExit) as raised:
        bootstrap.parse(["exec", "--fail-fast", "--", "sc-lint"])
    assert raised.value.code == 2


def test_old_python_fails_before_importing_the_bootstrap() -> None:
    probe = "import runpy,sys; sys.version_info=(3,10,0); runpy.run_path(sys.argv[1],run_name='__main__')"
    result = run_subprocess([sys.executable, "-c", probe, str(ASSET)], text=True, capture_output=True, check=False)
    assert result.returncode == 1
    assert "CLI.SC_LINT_PYTHON_UNAVAILABLE" in result.stderr


def build_fake_product(tmp_path: Path) -> Path:
    """A stand-in binary: compatible with any config, exits SC_LINT_TEST_EXIT on `lint`."""
    source = tmp_path / "fake.rs"
    source.write_text('fn main() { if std::env::args().any(|a| a == "lint") { std::process::exit(std::env::var("SC_LINT_TEST_EXIT").unwrap().parse().unwrap()); } }')
    binary = tmp_path / ("fake.exe" if os.name == "nt" else "fake")
    build = run_subprocess(["rustc", str(source), "-o", str(binary)], text=True, capture_output=True, check=False)
    assert build.returncode == 0, build.stderr
    return binary


@pytest.mark.integration
@pytest.mark.parametrize("exit_code", [0, 1, 2, 3, 4, 7])
def test_just_passes_through_product_exit_codes(exit_code: int, product_artifacts: ProductArtifacts, tmp_path: Path) -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    config = consumer / "sc-lint.toml"
    config.write_text(config.read_text().replace('minimum_version = "0.5.0"', f'minimum_version = "{product_artifacts.version}"'))
    binary = build_fake_product(tmp_path)
    environment = {**os.environ, "SC_LINT_BIN": str(binary), "SC_LINT_WHEEL_DIR": str(product_artifacts.wheel_dir), "SC_LINT_TEST_EXIT": str(exit_code)}
    result = run_subprocess(["just", "lint"], cwd=consumer, env=environment, text=True, capture_output=True, check=False, timeout=300)
    assert result.returncode == exit_code, result.stdout + result.stderr


@pytest.mark.integration
def test_just_reports_bootstrap_unreadable_minimum_version(product_artifacts: ProductArtifacts, tmp_path: Path) -> None:
    # Bootstrap's own exit 3 (not a child's): the compatible fake binary gets the
    # bootstrap past `ensure`, then `provision_venv` cannot read minimum_version.
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    config = consumer / "sc-lint.toml"
    lines = [line for line in config.read_text().splitlines() if not line.startswith("minimum_version")]
    assert len(lines) == len(config.read_text().splitlines()) - 1
    config.write_text("\n".join(lines) + "\n")
    binary = build_fake_product(tmp_path)
    environment = {**os.environ, "SC_LINT_BIN": str(binary), "SC_LINT_WHEEL_DIR": str(product_artifacts.wheel_dir), "SC_LINT_TEST_EXIT": "0"}
    result = run_subprocess(["just", "lint"], cwd=consumer, env=environment, text=True, capture_output=True, check=False, timeout=300)
    assert result.returncode == 3, result.stdout + result.stderr
    assert "CLI.SC_LINT_CONFIG_MALFORMED" in result.stderr, result.stderr
    assert "minimum_version" in result.stderr, result.stderr


@pytest.mark.parametrize("arguments", [["lint", "a", "b", "c"], ["test", "--list", "a"]])
def test_just_reports_bootstrap_usage_errors(arguments: list[str]) -> None:
    # Bootstrap's own exit 2 through a recipe: the installed Justfile forwards the
    # extra positionals, and the bootstrap rejects them before touching any binary.
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    result = run_subprocess(["just", *arguments], cwd=consumer, text=True, capture_output=True, check=False, timeout=300)
    assert result.returncode == 2, result.stdout + result.stderr
    usage_lines = [line for line in result.stderr.splitlines() if line.startswith("usage: .sc-lint/bootstrap.py")]
    assert len(usage_lines) == 1, result.stderr
    assert usage_lines[0].split("; ") == list(CC1D496_USAGE_GRAMMAR)


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
