"""Durable contract checks for the adoption skill and its documentation."""
from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "packages" / "sc-lint-adoption"
SKILL = PACKAGE / ".claude" / "skills" / "sc-lint-adoption" / "SKILL.md"
GUIDE = ROOT / "docs" / "sc-lint" / "adoption.md"


def test_marketplace_advertises_the_adoption_package() -> None:
    marketplace = json.loads((ROOT / ".claude-plugin" / "marketplace.json").read_text())
    plugin = next(item for item in marketplace["plugins"] if item["name"] == "sc-lint-adoption")
    assert plugin["source"] == "./packages/sc-lint-adoption"


def test_skill_has_the_seven_agent_run_steps() -> None:
    text = SKILL.read_text()
    for step in range(1, 8):
        assert f"## {step}." in text
    assert "just setup && just lint && just test" in text
    assert "sc-lint adoption dry-run exit" in text
    assert "command arrays must name a shipped binary or `sc_lint` module only" in text
    assert "test-<name>" in text
    assert "sc-lint docs --path" in text
    assert "adopt.xml.j2" in text


def test_package_assets_and_guide_stay_consumer_generic() -> None:
    for path in [*PACKAGE.joinpath(".claude").rglob("*"), GUIDE]:
        if path.is_file():
            content = path.read_text()
            assert "sc-compose" not in content
            assert "atm-core" not in content
            assert "wyvern" not in content


def test_adoption_guide_covers_the_contract_boundaries() -> None:
    text = GUIDE.read_text()
    for required in (
        "Consumer end state",
        "`install.json`",
        "Drift and safe application",
        "sc-publish",
        "How to extend",
        "Migrate existing named test recipes",
        "Offline documentation",
        "sc-lint docs --path",
        "sole repository-specific *policy*",
        "Assignment template",
        "command arrays must name a shipped binary or `sc_lint` module only",
    ):
        assert required in text


def test_established_fixture_exercises_named_test_recipe_migration() -> None:
    fixture = ROOT / "tests" / "fixtures" / "adoption" / "established-workspace"
    justfile = (fixture / "Justfile").read_text()
    install = json.loads((fixture / "install.json").read_text())
    assert "test-integration:\n    just test integration" in justfile
    assert install["test"]["integration"] == ["cargo", "test", "--test", "integration"]
