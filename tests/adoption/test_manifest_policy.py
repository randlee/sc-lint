"""Installer coverage for optional boundary manifest-policy records."""
from __future__ import annotations

import copy
import json
import runpy
import tomllib
from pathlib import Path
from typing import Any

import pytest
from conftest import ProductArtifacts
from install_helpers import FIXTURES
from install_helpers import KIT
from install_helpers import copy_fixture
from install_helpers import run
from install_helpers import run_subprocess

POLICY_FIXTURE = FIXTURES / "manifest-policy.json"
POLICY_KEYS = {
    "manifest_dependency_allowlists",
    "global_dependency_ownership",
    "manifest_section_rules",
}


def installer_schema() -> dict[str, Any]:
    return runpy.run_path(str(KIT / "install.py"), run_name="sc_lint_adoption_install")["load_schema"]()


def policy_input(consumer: Path, policy: dict[str, Any] | None = None) -> tuple[Path, dict[str, Any]]:
    data = json.loads((FIXTURES / "install.json").read_text())
    selected_policy = copy.deepcopy(policy if policy is not None else json.loads(POLICY_FIXTURE.read_text()))
    data["analyzers"]["boundary"] = selected_policy
    path = consumer.parent / "install.json"
    path.write_text(json.dumps(data))
    return path, selected_policy


def mutate_policy(policy: dict[str, Any], case: str) -> None:
    collection, field = case.split(".", 1)
    if field == "missing_owner":
        policy[collection][0].pop("owner_manifest_path")
    elif field == "wrong_owner_type":
        policy[collection][0]["owner_manifest_path"] = 7
    elif field == "missing_allowed":
        policy[collection][0].pop("allowed_dependencies")
    elif field == "wrong_allowed_type":
        policy[collection][0]["allowed_dependencies"] = "serde"
    elif field == "blank_owner":
        policy[collection][0]["owner_manifest_path"] = "  "
    elif field == "blank_item":
        policy[collection][0]["allowed_dependencies"] = [""]
    elif field == "wrong_item_type":
        policy[collection][0]["allowed_dependencies"] = [7]
    elif field == "bad_optional_shape":
        policy[collection][0]["boundary_record_path"] = 7
    elif field == "bad_optional_item":
        policy[collection][0]["boundary_record_path"] = [""]
    elif field == "missing_dependency":
        policy[collection][0].pop("dependency")
    elif field == "wrong_dependency_type":
        policy[collection][0]["dependency"] = []
    elif field == "blank_dependency":
        policy[collection][0]["dependency"] = " "
    elif field == "blank_message":
        policy[collection][0]["manifest_message"] = ""
    elif field == "blank_source_message":
        policy[collection][0]["source_message"] = " "
    elif field == "missing_source_message":
        policy[collection][0].pop("source_message")
    elif field == "bad_optional_array":
        policy[collection][0]["allowed_manifest_paths"] = "crates/app/Cargo.toml"
    elif field == "bad_optional_array_item":
        policy[collection][0]["allowed_source_roots"] = [7]
    elif field == "blank_optional_array_item":
        policy[collection][0]["allowed_manifest_paths"] = [""]
    elif field == "missing_package":
        policy[collection][0].pop("dependency_package")
    elif field == "blank_package":
        policy[collection][0]["dependency_package"] = ""
    elif field == "blank_section_owner":
        policy[collection][0]["owner_manifest_path"] = " "
    elif field == "wrong_section_owner_type":
        policy[collection][0]["owner_manifest_path"] = 1
    elif field == "blank_section_message":
        policy[collection][0]["message"] = ""
    elif field == "bad_sections":
        policy[collection][0]["allowed_sections"] = "dependencies"
    elif field == "bad_section_item":
        policy[collection][0]["allowed_sections"] = [""]
    elif field == "wrong_section_item_type":
        policy[collection][0]["allowed_sections"] = [7]
    else:
        raise AssertionError(f"unknown policy mutation: {case}")


@pytest.mark.integration
def test_issue136_installer_roundtrip(product_artifacts: ProductArtifacts) -> None:
    fragment = json.loads((KIT / "schema.d" / "manifest-policy.json").read_text())
    schema = installer_schema()
    boundary_schema = schema["properties"]["analyzers"]["properties"]["boundary"]
    assert POLICY_KEYS <= set(fragment["properties"]["analyzers"]["properties"]["boundary"]["properties"])
    assert POLICY_KEYS <= set(boundary_schema["properties"])

    consumer = copy_fixture("empty-workspace")
    boundaries = consumer / "boundaries"
    boundaries.mkdir()
    (boundaries / "planning.toml").write_text('[planning]\ncurrent_sprint = "K.4"\n')
    owner_boundaries = boundaries / "empty-workspace"
    owner_boundaries.mkdir()
    (owner_boundaries / "empty-workspace.toml").write_text(
        (FIXTURES / "manifest-policy-boundary.toml").read_text()
    )
    cargo_manifest = consumer / "Cargo.toml"
    cargo_manifest.write_text(
        cargo_manifest.read_text()
        + '\n[workspace]\nmembers = ["."]\nresolver = "2"\n'
        + '\n[workspace.package]\nversion = "0.1.0"\n'
    )
    input_path, expected_policy = policy_input(consumer)
    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 0, result.stderr

    rendered = tomllib.loads((consumer / "sc-lint.toml").read_text())
    parsed_policy = rendered["tool"]["sc-lint"]["analyzers"]["boundary"]
    assert parsed_policy == expected_policy
    assert json.loads((consumer / ".sc-lint" / "install.json").read_text())["analyzers"]["boundary"] == expected_policy

    rust_parse = run_subprocess(
        [
            str(product_artifacts.binary),
            "--json",
            "--root",
            str(consumer),
            "--config",
            str(consumer / "sc-lint.toml"),
            "lint",
            "sc-boundary",
            "--rule-filter",
            "SCB-MANIFEST-003,SCB-MANIFEST-004,SCB-MANIFEST-005,SCB-MANIFEST-006",
        ],
        cwd=consumer,
        text=True,
        capture_output=True,
        check=False,
    )
    assert rust_parse.returncode == 0, rust_parse.stdout + rust_parse.stderr
    report = json.loads(rust_parse.stdout)
    assert report["data"]["status"] == "pass", report


@pytest.mark.parametrize(
    ("case", "error_fragment"),
    [
        ("manifest_dependency_allowlists.missing_owner", "missing required fields"),
        ("manifest_dependency_allowlists.wrong_owner_type", "must be string"),
        ("manifest_dependency_allowlists.missing_allowed", "missing required fields"),
        ("manifest_dependency_allowlists.wrong_allowed_type", "must be array"),
        ("manifest_dependency_allowlists.blank_owner", "does not match"),
        ("manifest_dependency_allowlists.blank_item", "does not match"),
        ("manifest_dependency_allowlists.wrong_item_type", "must be string"),
        ("manifest_dependency_allowlists.bad_optional_shape", "match exactly one accepted shape"),
        ("manifest_dependency_allowlists.bad_optional_item", "does not match"),
        ("global_dependency_ownership.missing_dependency", "missing required fields"),
        ("global_dependency_ownership.wrong_dependency_type", "must be string"),
        ("global_dependency_ownership.blank_dependency", "does not match"),
        ("global_dependency_ownership.blank_message", "does not match"),
        ("global_dependency_ownership.blank_source_message", "does not match"),
        ("global_dependency_ownership.missing_source_message", "missing required fields"),
        ("global_dependency_ownership.bad_optional_array", "must be array"),
        ("global_dependency_ownership.bad_optional_array_item", "must be string"),
        ("global_dependency_ownership.blank_optional_array_item", "does not match"),
        ("manifest_section_rules.missing_package", "missing required fields"),
        ("manifest_section_rules.blank_package", "does not match"),
        ("manifest_section_rules.blank_section_owner", "does not match"),
        ("manifest_section_rules.wrong_section_owner_type", "must be string"),
        ("manifest_section_rules.blank_section_message", "does not match"),
        ("manifest_section_rules.bad_sections", "must be array"),
        ("manifest_section_rules.bad_section_item", "does not match"),
        ("manifest_section_rules.wrong_section_item_type", "must be string"),
    ],
)
def test_issue136_installer_rejects_structural_policy_errors(case: str, error_fragment: str) -> None:
    consumer = copy_fixture("empty-workspace")
    policy = json.loads(POLICY_FIXTURE.read_text())
    mutate_policy(policy, case)
    input_path, _ = policy_input(consumer, policy)

    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 2
    assert error_fragment in result.stderr, result.stderr
    assert not (consumer / "sc-lint.toml").exists()
    assert not (consumer / ".sc-lint").exists()


def test_issue136_installer_preserves_empty_policy_families() -> None:
    consumer = copy_fixture("empty-workspace")
    policy = {key: [] for key in POLICY_KEYS}
    input_path, _ = policy_input(consumer, policy)
    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 0, result.stderr


def test_issue136_installer_accepts_empty_allowlists_and_sections() -> None:
    consumer = copy_fixture("empty-workspace")
    policy = json.loads(POLICY_FIXTURE.read_text())
    policy["manifest_dependency_allowlists"][0]["allowed_dependencies"] = []
    policy["manifest_section_rules"][0]["allowed_sections"] = []
    input_path, expected_policy = policy_input(consumer, policy)
    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 0, result.stderr
    rendered = tomllib.loads((consumer / "sc-lint.toml").read_text())
    assert rendered["tool"]["sc-lint"]["analyzers"]["boundary"] == expected_policy


def test_issue136_schema_fragment_only_extends_boundary() -> None:
    schema = installer_schema()
    analyzers = schema["properties"]["analyzers"]["properties"]
    assert POLICY_KEYS <= set(analyzers["boundary"]["properties"])
