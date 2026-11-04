"""Managed consumer policy survives kit installation and repeated checks."""
from __future__ import annotations

import json
import sys
from sc_lint._toml import loads as toml_loads

import pytest
from conftest import ProductArtifacts
from install_helpers import copy_fixture, run, run_subprocess, write_input

POLICIES = {
    "line_counts": {"max_production_lines": 1200},
    "identities": {"forbidden_literals": ["example"]},
    "version_sync": {"python": {"cargo_lock": True}},
    "cargo_shear": {"allowed_unused_dependencies": {}},
    "logging": {"root": "reports/logs", "console": False},
    "artifacts": {"root": "reports/artifacts"},
}


def install_policy(consumer, version="0.8.4"):
    return write_input(consumer, minimum_version=version, **POLICIES,
                       default_lint_profile="quick", lint_profiles={"quick": ["z-first"]},
                       profiles={"z-first": [sys.executable, "-c", "pass"], "a-next": [sys.executable, "-c", "raise SystemExit(9)"]})


def test_helper_policies_and_default_profile_roundtrip_without_drift():
    consumer = copy_fixture("empty-workspace")
    source = install_policy(consumer)
    result = run("--input", str(source), str(consumer))
    assert result.returncode == 0, result.stderr
    rendered = toml_loads((consumer / "sc-lint.toml").read_text())
    assert {key: rendered[key] for key in POLICIES} == POLICIES
    assert rendered["tool"]["sc-lint"]["default_lint_profile"] == "quick"
    assert [row["name"] for row in rendered["tool"]["sc-lint"]["lint"]] == ["z-first", "a-next"]
    for arguments in [("--dry-run",), ()]:
        result = run(*arguments, "--input", str(consumer / ".sc-lint/install.json"), str(consumer))
        assert result.returncode == 0, result.stdout + result.stderr
    assert json.loads((consumer / ".sc-lint/install.json").read_text())["logging"] == POLICIES["logging"]


@pytest.mark.parametrize("field,value", [("line_counts", []), ("logging", "bad"),
    ("default_lint_profile", ""), ("version_sync", {"python": None})])
def test_invalid_helper_policy_input_does_not_write_config(field, value):
    consumer = copy_fixture("empty-workspace")
    source = write_input(consumer, **{field: value})
    result = run("--input", str(source), str(consumer))
    assert result.returncode == 2, result.stdout + result.stderr
    assert not (consumer / "sc-lint.toml").exists()


@pytest.mark.integration
def test_product_accepts_kit_helper_policies_and_default_profile(product_artifacts: ProductArtifacts):
    consumer = copy_fixture("empty-workspace")
    source = install_policy(consumer, product_artifacts.version)
    installed = run("--input", str(source), str(consumer))
    assert installed.returncode == 0, installed.stderr
    result = run_subprocess([str(product_artifacts.binary), "--json", "--config", str(consumer / "sc-lint.toml"),
                             "lint", "ci", "--consumer"], cwd=consumer, text=True, capture_output=True)
    assert result.returncode == 0, result.stdout + result.stderr
    assert run("--dry-run", "--input", str(consumer / ".sc-lint/install.json"), str(consumer)).returncode == 0
