"""Platform-aware lint profiles through the adoption installer (ADR-017 change 4).

The installer checks JSON shape only and renders the accepted values verbatim;
the sc-lint binary owns every semantic rule (members, platform, target, command
shape). These tests pin both halves of that contract.
"""
from __future__ import annotations

import json
import os
import sys
import tomllib
from pathlib import Path

import pytest
from conftest import ProductArtifacts
from install_helpers import FIXTURES
from install_helpers import ROOT
from install_helpers import WORKED_EXAMPLE_TIMEOUT_SECONDS
from install_helpers import copy_fixture
from install_helpers import run
from install_helpers import run_subprocess

PROFILE_FIXTURES = ROOT / "crates" / "sc-lint" / "tests" / "fixtures" / "platform-profile-execution"
SEMANTIC_INVALID = PROFILE_FIXTURES / "semantic-invalid"
HOST_PLATFORM = {"darwin": "macos", "win32": "windows"}.get(sys.platform, "linux")


def install_profiles(profiles: dict, lint_profiles: dict | None, *, ci_os: list[str] | None = None) -> tuple[Path, "object"]:
    consumer = copy_fixture("empty-workspace")
    data = json.loads((FIXTURES / "install.json").read_text())
    data["profiles"] = profiles
    if lint_profiles is not None:
        data["lint_profiles"] = lint_profiles
    if ci_os is not None:
        data["ci"]["os"] = ci_os
    path = consumer.parent / "install.json"
    path.write_text(json.dumps(data))
    return consumer, run("--input", str(path), str(consumer))


def rendered_config(consumer: Path) -> dict:
    return tomllib.loads((consumer / "sc-lint.toml").read_text())["tool"]["sc-lint"]


def test_profiles_exact_json_toml_roundtrip() -> None:
    fragment = json.loads((PROFILE_FIXTURES / "roundtrip-fragment.json").read_text())
    expected = tomllib.loads((PROFILE_FIXTURES / "roundtrip-fragment.toml").read_text())["tool"]["sc-lint"]
    consumer, result = install_profiles(fragment["profiles"], fragment["lint_profiles"])
    assert result.returncode == 0, result.stderr

    config = rendered_config(consumer)
    assert config["lint_profiles"] == expected["lint_profiles"]
    assert config["lint"] == expected["lint"]
    # Order is part of the contract: profile membership and step declaration order.
    assert list(config["lint_profiles"]["ci"]) == ["format", "native-check", "windows-check"]
    assert [step["name"] for step in config["lint"]] == ["format", "native-check", "windows-check"]
    assert config["lint"][2] == {
        "name": "windows-check",
        "command": ["cargo", "check", "--workspace"],
        "platform": "windows",
        "target": "x86_64-pc-windows-msvc",
    }
    assert "platform" not in config["lint"][0], "array steps stay untagged"
    text = (consumer / "sc-lint.toml").read_text()
    assert text.index("[tool.sc-lint.lint_profiles]") < text.index("[[tool.sc-lint.lint]]")


def test_absent_lint_profiles_renders_no_profile_table() -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    assert "lint_profiles" not in (consumer / "sc-lint.toml").read_text()


@pytest.mark.parametrize("case", sorted(json.loads((SEMANTIC_INVALID / "expected.json").read_text())))
def test_installer_shape_only_rust_semantic_rejection(case: str) -> None:
    """Semantically invalid but shape-valid input is accepted and rendered verbatim.

    Rejection belongs to the Rust loader, which the sc-lint crate tests pin to
    the same fixtures and the PROFILE.* detail code in expected.json.
    """
    fragment = json.loads((SEMANTIC_INVALID / f"{case}.json").read_text())
    consumer, result = install_profiles(fragment["profiles"], fragment.get("lint_profiles"))
    assert result.returncode == 0, f"{case}: {result.stderr}"
    expected = tomllib.loads((SEMANTIC_INVALID / f"{case}.toml").read_text())["tool"]["sc-lint"]
    config = rendered_config(consumer)
    assert config["lint"] == expected["lint"], case
    assert config.get("lint_profiles") == expected.get("lint_profiles"), case


@pytest.mark.parametrize(
    ("profiles", "lint_profiles", "fragment"),
    [
        ({"a": ["cargo", "check"]}, {"ci": "a"}, "lint_profiles.ci must be an array"),
        ({"a": ["cargo", "check"]}, {"ci": [1]}, "lint_profiles.ci[0] must be a string"),
        ({"a": ["cargo", "check"]}, ["a"], "lint_profiles must be an object"),
        ({"a": 7}, None, "profiles.a must match exactly one accepted shape"),
        ({"a": {"command": ["cargo"], "extra": "x"}}, None, "unknown fields"),
        ({"a": {"platform": "linux"}}, None, "missing required fields"),
        ({"a": {"command": "cargo check"}}, None, "must be an array"),
        ({"a": {"command": ["cargo"], "platform": 3}}, None, "must be a string"),
        ({"a": {"command": ["cargo"], "target": ["x"]}}, None, "must be a string"),
    ],
)
def test_installer_rejects_wrong_shapes(profiles: dict, lint_profiles: object, fragment: str) -> None:
    consumer, result = install_profiles(profiles, lint_profiles)  # type: ignore[arg-type]
    assert result.returncode != 0
    assert fragment in result.stderr, result.stderr
    assert not (consumer / "sc-lint.toml").exists(), "a rejected input writes nothing"


def lint_lines(consumer: Path) -> list[str]:
    lines = (consumer / ".github" / "workflows" / "sc-lint.yml").read_text().splitlines()
    start = lines.index("      - run: just setup") + 1
    end = lines.index("      - run: just test")
    return lines[start:end]


def test_ci_workflow_resolution_golden() -> None:
    plain = {"fmt": ["cargo", "fmt", "--check"]}
    # A declared `ci` profile: every platform on Linux, host-only elsewhere.
    consumer, result = install_profiles(plain, {"ci": ["fmt"]}, ci_os=["linux", "macos", "windows"])
    assert result.returncode == 0, result.stderr
    assert lint_lines(consumer) == [
        "      - run: just lint ci all",
        "        if: runner.os == 'Linux'",
        "      - run: just lint ci",
        "        if: runner.os != 'Linux'",
    ]
    # A lone step named `ci` keeps its single-step invocation.
    consumer, result = install_profiles({"ci": ["cargo", "check"]}, None)
    assert result.returncode == 0, result.stderr
    assert lint_lines(consumer) == ["      - run: just lint ci"]
    # Neither: the unchanged whole-config invocation.
    consumer, result = install_profiles(plain, {"fast": ["fmt"]})
    assert result.returncode == 0, result.stderr
    assert lint_lines(consumer) == ["      - run: just lint"]


@pytest.mark.integration
def test_consumer_profile_installed_wheel_and_binary(product_artifacts: ProductArtifacts) -> None:
    profiles = {
        "fmt": ["cargo", "--version"],
        "probe": ["rustc", "--version"],
        "windows-probe": {"command": ["rustc", "--version"], "platform": "windows"},
    }
    consumer, result = install_profiles(profiles, {"ci": ["fmt", "probe", "windows-probe"]})
    assert result.returncode == 0, result.stderr
    config = (consumer / "sc-lint.toml").read_text()
    (consumer / "sc-lint.toml").write_text(
        config.replace('minimum_version = "0.5.0"', f'minimum_version = "{product_artifacts.version}"')
    )
    environment = {
        **os.environ,
        "SC_LINT_BIN": str(product_artifacts.binary),
        "SC_LINT_WHEEL_DIR": str(product_artifacts.wheel_dir),
    }

    def just_lint(*arguments: str) -> "object":
        return run_subprocess(
            ["just", "lint", *arguments],
            cwd=consumer,
            env=environment,
            text=True,
            capture_output=True,
            check=False,
            timeout=WORKED_EXAMPLE_TIMEOUT_SECONDS,
        )

    def rows(scope: str) -> list[dict]:
        # `just lint` prints human text; the structured rows come from the same
        # installed binary the bootstrap dispatches to.
        command = run_subprocess(
            [str(product_artifacts.binary), "--json", "--config", "sc-lint.toml", "lint", "ci", "--consumer",
             "--profile", "ci", "--platform-scope", scope],
            cwd=consumer,
            text=True,
            capture_output=True,
            check=False,
            timeout=WORKED_EXAMPLE_TIMEOUT_SECONDS,
        )
        assert command.returncode == 0, f"{scope}: {command.stdout}\n{command.stderr}"
        return json.loads(command.stdout)["data"]["steps"]

    for arguments in (("ci",), ("ci", "all"), ("ci", "host")):
        command = just_lint(*arguments)
        assert command.returncode == 0, f"just lint {arguments}: {command.stdout}\n{command.stderr}"
        assert "lint.ci.consumer: ok" in command.stdout

    host_names = ["fmt", "probe", "windows-probe"] if os.name == "nt" else ["fmt", "probe"]
    assert [step["name"] for step in rows("host")] == host_names
    every = rows("all")
    assert [step["name"] for step in every] == ["fmt", "probe", "windows-probe"]
    if os.name != "nt":
        assert every[2] == {
            "status": "skipped", "name": "windows-probe", "host": HOST_PLATFORM,
            "platform": "windows", "target": None, "reason": "unsupported_cross_command",
        }

    # Planted negatives: a bad scope is a usage error and an unknown selector a config error.
    assert just_lint("ci", "nowhere").returncode == 2
    assert just_lint("no-such-selector").returncode == 3
