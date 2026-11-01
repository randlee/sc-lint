"""Keep the vendorable profile schema aligned with the Rust step parser."""
from __future__ import annotations

import json
import re
import runpy
import tomllib
from pathlib import Path
from typing import Any

import pytest
from install_helpers import KIT
from install_helpers import ROOT
from install_helpers import copy_fixture
from install_helpers import run
from install_helpers import write_input

RUST_CONFIG = ROOT / "crates" / "sc-lint" / "src" / "config.rs"
RUST_PROFILE_PLAN = ROOT / "crates" / "sc-lint" / "src" / "profile_plan.rs"


def installer_schema() -> dict[str, Any]:
    return runpy.run_path(str(KIT / "install.py"), run_name="sc_lint_adoption_install")["load_schema"]()


def rust_step_fields(source: str) -> set[str]:
    match = re.search(r"struct ConsumerProfileStepFile\s*\{(?P<body>.*?)^\}", source, re.M | re.S)
    assert match is not None, "Rust consumer step model was not found"
    return set(re.findall(r"^\s*(\w+)\s*:", match["body"], re.M)) - {"extra"}


def rust_test_only_fields(config_source: str, plan_source: str) -> set[str]:
    rejected = set()
    test_rejection = plan_source.split("pub(crate) fn reject_test_step_fields(", 1)[1]
    test_rejection = test_rejection.split("/// Validated ordered lint profiles", 1)[0]
    rejected.update(re.findall(r"\.then_some\(\"([a-z_]+)\"\)", test_rejection))

    execution = config_source.split("fn profile_step_execution(", 1)[1]
    test_branch = execution.split("let parse_error", 1)[0]
    rejected.update(re.findall(r"step\.(\w+)\.is_some\(\)", test_branch))
    return rejected


def test_profile_and_test_schemas_match_rust_step_fields() -> None:
    schema = installer_schema()
    source = RUST_CONFIG.read_text()
    plan_source = RUST_PROFILE_PLAN.read_text()
    rust_fields = rust_step_fields(source)
    lint_options = schema["properties"]["profiles"]["additionalProperties"]["oneOf"]
    lint_step = next(option for option in lint_options if option.get("type") == "object")
    assert set(lint_step["properties"]) == rust_fields
    assert lint_step["additionalProperties"] is False

    rust_test_fields = rust_fields - rust_test_only_fields(source, plan_source)
    test_options = schema["properties"]["test"]["additionalProperties"]["oneOf"]
    test_step = next(option for option in test_options if option.get("type") == "object")
    # Install-input map keys provide the Rust row's `name` field.
    assert set(test_step["properties"]) | {"name"} == rust_test_fields
    assert test_step["additionalProperties"] is False


def test_profile_schema_enforces_rust_step_value_constraints() -> None:
    install = runpy.run_path(str(KIT / "install.py"), run_name="sc_lint_adoption_install")
    validate = install["_schema_value"]
    schema = installer_schema()
    lint_options = schema["properties"]["profiles"]["additionalProperties"]["oneOf"]
    lint_step = next(option for option in lint_options if option.get("type") == "object")
    valid = {
        "name": "clippy",
        "command": ["cargo", "clippy"],
        "platform": "linux",
        "after_all": True,
        "env": {"PYO3_PYTHON": ".bootstrap-venv/bin/python"},
        "cwd": "crates/atm-core",
    }
    validate(valid, lint_step, "profiles.clippy")

    invalid_values = [
        ("name", "  "),
        ("platform", "freebsd"),
        ("target", 7),
        ("after_all", "true"),
        ("env", {"RUST_LOG": 1}),
        ("cwd", "../outside"),
        ("cwd", "\\absolute"),
        ("command", []),
        ("command", [""]),
    ]
    for field, value in invalid_values:
        with pytest.raises(ValueError):
            validate({**valid, field: value}, lint_step, "profiles.clippy")

    test_options = schema["properties"]["test"]["additionalProperties"]["oneOf"]
    test_step = next(option for option in test_options if option.get("type") == "object")
    validate({"command": ["cargo", "test"], "after_all": True}, test_step, "test.unit")
    with pytest.raises(ValueError):
        validate({"command": ["cargo", "test"], "env": {}}, test_step, "test.unit")


def test_install_accepts_and_renders_schema_valid_profile_fields() -> None:
    consumer = copy_fixture("empty-workspace")
    profiles = {
        "clippy": {
            "name": "clippy",
            "command": ["cargo", "clippy"],
            "env": {"PYO3_PYTHON": ".bootstrap-venv/bin/python"},
            "cwd": "crates/atm-core",
        }
    }
    test_layers = {"unit": {"command": ["cargo", "test"], "after_all": True}}
    input_path = write_input(consumer, profiles=profiles, test=test_layers)

    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 0, result.stderr

    installed_fragment = consumer / "plugins" / "sc-lint" / "schema.d" / "profiles.json"
    assert json.loads(installed_fragment.read_text()) == json.loads((KIT / "schema.d" / "profiles.json").read_text())
    config = tomllib.loads((consumer / "sc-lint.toml").read_text())["tool"]["sc-lint"]
    assert config["lint"] == [
        {
            "name": "clippy",
            "command": ["cargo", "clippy"],
            "env": {"PYO3_PYTHON": ".bootstrap-venv/bin/python"},
            "cwd": "crates/atm-core",
        }
    ]
    assert config["test"] == [{"name": "unit", "command": ["cargo", "test"], "after_all": True}]
