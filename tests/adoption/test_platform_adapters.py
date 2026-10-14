"""Installer coverage for optional portability platform-adapter records."""
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


def test_issue372_installer_roundtrip_preserves_inline_adapter_records() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = input_with_adapters(consumer, ADAPTERS)
    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 0, result.stderr
    rendered = (consumer / "sc-lint.toml").read_text()
    assert "platform_adapters = [{" in rendered
    assert "unix_module" in rendered
    assert run("--dry-run", "--input", str(input_path), str(consumer)).returncode == 0


@pytest.mark.parametrize("adapters", [
    {},
    [{"unix_module": "a.rs", "windows_module": "b.rs"}],
    [{"unix_module": "", "windows_module": "b.rs", "reason": "reason"}],
    [{"unix_module": "a.rs", "windows_module": "b.rs", "reason": "reason", "extra": "x"}],
])
def test_issue372_installer_rejects_structural_adapter_errors(adapters: object) -> None:
    consumer = copy_fixture("empty-workspace")
    result = run("--input", str(input_with_adapters(consumer, adapters)), str(consumer))
    assert result.returncode == 2
    assert not (consumer / ".sc-lint").exists()


def test_issue372_schema_fragment_only_extends_portability() -> None:
    schema = installer_schema()
    assert schema["properties"]["analyzers"]["properties"]["portability"]["properties"]["platform_adapters"]["type"] == "array"
