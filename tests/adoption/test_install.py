"""Integration tests for the vendorable adoption kit."""
from __future__ import annotations

import importlib.util
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
KIT = ROOT / "packages" / "sc-lint-adoption"
INSTALL = KIT / "install.py"
FIXTURES = ROOT / "tests" / "fixtures" / "adoption"


def copy_fixture(name: str) -> Path:
    temporary = Path(tempfile.mkdtemp())
    destination = temporary / "consumer"
    shutil.copytree(FIXTURES / name, destination)
    return destination


def run(*arguments: str, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["python3", str(INSTALL), *arguments], cwd=cwd, text=True, capture_output=True, check=False
    )


def test_empty_workspace_installs_converges_and_exposes_consumer_recipes() -> None:
    consumer = copy_fixture("empty-workspace")
    install = run("--input", str(FIXTURES / "install.json"), str(consumer))
    assert install.returncode == 0, install.stderr
    clean = run("--dry-run", "--input", str(FIXTURES / "install.json"), str(consumer))
    assert clean.returncode == 0, clean.stdout
    assert os.access(consumer / ".sc-lint" / "bootstrap", os.X_OK)
    recipes = subprocess.run(["just", "--list"], cwd=consumer, text=True, capture_output=True, check=False)
    assert recipes.returncode == 0, recipes.stderr
    for recipe in ("setup", "lint", "test", "upgrade"):
        assert f"    {recipe}" in recipes.stdout


def test_established_justfile_preserves_unmanaged_recipe() -> None:
    consumer = copy_fixture("established-workspace")
    before = (consumer / "Justfile").read_text()
    result = run("--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 0, result.stderr
    rendered = (consumer / "Justfile").read_text()
    assert before in rendered
    assert rendered.count("# >>> sc-lint managed integration >>>") == 1


def test_dry_run_reports_modified_managed_asset() -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    managed = consumer / ".sc-lint" / "justfile"
    managed.write_text(managed.read_text() + "# consumer edit\n")
    result = run("--dry-run", "--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 1
    assert str(managed) in result.stdout


def test_install_rejects_a_modified_managed_asset_without_writing() -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    managed = consumer / ".sc-lint" / "justfile"
    managed.write_text(managed.read_text() + "# consumer edit\n")
    before = managed.read_bytes()
    result = run("--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 2
    assert str(managed) in result.stderr
    assert managed.read_bytes() == before


def test_vendored_kit_can_recheck_and_schema_stays_vendored() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = FIXTURES / "install.json"
    assert run("--input", str(input_path), str(consumer)).returncode == 0
    installer = consumer / "plugins" / "sc-lint" / "install.py"
    assert installer.is_file()
    assert not (consumer / "install.schema.json").exists()
    result = subprocess.run(["python3", str(installer), "--dry-run", "--input", str(input_path), str(consumer)], text=True, capture_output=True, check=False)
    assert result.returncode == 0, result.stderr


def test_duplicate_markers_are_conflicts_without_writes() -> None:
    consumer = copy_fixture("empty-workspace")
    justfile = consumer / "Justfile"
    justfile.write_text("# >>> sc-lint managed integration >>>\n# <<< sc-lint managed integration <<<\n" * 2)
    before = justfile.read_bytes()
    result = run("--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 2
    assert str(justfile) in result.stderr or "marker conflict" in result.stderr
    assert justfile.read_bytes() == before


def test_analyzer_worked_example_is_declarative() -> None:
    consumer = copy_fixture("analyzer-worked-example")
    result = run("--input", str(FIXTURES / "analyzer-worked-example" / "install.json"), str(consumer))
    assert result.returncode == 0, result.stderr
    config = (consumer / "sc-lint.toml").read_text()
    assert 'reason = "no async runtime"' in config
    assert '"linux"' in config
    assert 'name = "unit"' in config and 'name = "integrate"' in config


def test_analyzer_worked_example_runs_all_declared_just_layers() -> None:
    binary = ROOT / "target" / "debug" / "sc-lint"
    wheels = ROOT / ".sc-lint" / "wheels"
    if not binary.exists() or not wheels.exists():
        import pytest

        pytest.skip("requires the source product binary and offline wheel set")
    consumer = copy_fixture("analyzer-worked-example")
    result = run("--input", str(FIXTURES / "analyzer-worked-example" / "install.json"), str(consumer))
    assert result.returncode == 0, result.stderr
    environment = {**os.environ, "SC_LINT_BIN": str(binary), "SC_LINT_WHEEL_DIR": str(wheels)}
    for arguments in (("test",), ("test", "all"), ("test", "integrate"), ("lint",)):
        command = subprocess.run(["just", *arguments], cwd=consumer, env=environment, text=True, capture_output=True, check=False)
        assert command.returncode == 0, command.stderr
