"""Installer regressions for the optional declared-tools input (ADR-017 change 6).

The `tools` schema is supplied only by `schema.d/tools.json` through the
installer's existing fragment loader; the common installer and base schema are
unchanged.
"""
from __future__ import annotations

import json
import runpy
from pathlib import Path
from typing import Any

import pytest
from install_helpers import FIXTURES
from install_helpers import KIT
from install_helpers import copy_fixture
from install_helpers import run
from install_helpers import write_input

TOOLS = {"cargo": {"cargo-xwin": "0.18.4"}, "rustup_targets": ["x86_64-pc-windows-msvc"]}


def installer_schema() -> dict[str, Any]:
    # run_path compiles from source, so no bytecode cache lands in the kit.
    return runpy.run_path(str(KIT / "install.py"), run_name="sc_lint_adoption_install")["load_schema"]()


def persisted(consumer: Path) -> dict[str, Any]:
    return json.loads((consumer / ".sc-lint" / "install.json").read_text())


def test_tools_schema_comes_from_the_fragment_loader() -> None:
    fragment = json.loads((KIT / "schema.d" / "tools.json").read_text())
    base = json.loads((KIT / "install.schema.json").read_text())
    assert "tools" not in base["properties"]
    assert "tools" not in base["required"]
    merged = installer_schema()
    assert merged["properties"]["tools"] == fragment["properties"]["tools"]
    assert merged["required"] == base["required"]


def test_declared_tools_are_accepted_and_persisted_verbatim() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = write_input(consumer, tools=TOOLS)
    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 0, result.stderr
    assert persisted(consumer)["tools"] == TOOLS
    clean = run("--dry-run", "--input", str(input_path), str(consumer))
    assert clean.returncode == 0, clean.stdout + clean.stderr


@pytest.mark.parametrize(
    "tools",
    [{}, {"cargo": {}}, {"rustup_targets": []}, {"cargo": {}, "rustup_targets": []}],
)
def test_empty_or_partial_tools_are_accepted(tools: dict[str, Any]) -> None:
    consumer = copy_fixture("empty-workspace")
    result = run("--input", str(write_input(consumer, tools=tools)), str(consumer))
    assert result.returncode == 0, result.stderr
    assert persisted(consumer)["tools"] == tools


def test_absent_tools_keep_existing_installed_output() -> None:
    consumer = copy_fixture("empty-workspace")
    result = run("--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 0, result.stderr
    assert "tools" not in persisted(consumer)
    assert persisted(consumer) == json.loads((FIXTURES / "install.json").read_text())


@pytest.mark.parametrize(
    ("tools", "error_fragment"),
    [
        ([], "tools must be an object"),
        ({"pip": {"ruff": "0.1.0"}}, "tools has unknown fields: ['pip']"),
        ({"cargo": ["cargo-xwin"]}, "tools.cargo must be an object"),
        ({"cargo": {"cargo-xwin": 18}}, "tools.cargo.cargo-xwin must be a string"),
        ({"cargo": {"cargo-xwin": "0.18"}}, "tools.cargo.cargo-xwin does not match"),
        ({"cargo": {"cargo-xwin": "^0.18.4"}}, "tools.cargo.cargo-xwin does not match"),
        ({"cargo": {"cargo-xwin": "0.18.4-beta.1"}}, "tools.cargo.cargo-xwin does not match"),
        ({"cargo": {"cargo-xwin": "01.18.4"}}, "tools.cargo.cargo-xwin does not match"),
        ({"rustup_targets": "x86_64-pc-windows-msvc"}, "tools.rustup_targets must be an array"),
        ({"rustup_targets": [""]}, "tools.rustup_targets[0] does not match"),
        ({"rustup_targets": ["x86_64 pc"]}, "tools.rustup_targets[0] does not match"),
        ({"rustup_targets": [7]}, "tools.rustup_targets[0] must be a string"),
    ],
)
def test_malformed_tools_are_rejected_before_writing(tools: Any, error_fragment: str) -> None:
    consumer = copy_fixture("empty-workspace")
    result = run("--input", str(write_input(consumer, tools=tools)), str(consumer))
    assert result.returncode == 2
    assert error_fragment in result.stderr
    assert not (consumer / ".sc-lint").exists()
