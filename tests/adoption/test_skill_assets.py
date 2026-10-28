"""Durable contract checks for the adoption skill and its documentation."""
from __future__ import annotations

import json
from pathlib import Path
import re

import pytest


ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "packages" / "sc-lint-adoption"
SKILL = PACKAGE / ".claude" / "skills" / "sc-lint-adoption" / "SKILL.md"
GUIDE = ROOT / "docs" / "sc-lint" / "adoption.md"
ASSIGNMENT_TEMPLATE = SKILL.parent / "adopt.xml.j2"
TEMPLATE_EXPRESSION = re.compile(r"\{\{.*?\}\}|\{%.*?%\}|\{#.*?#\}", re.S)
PLAIN_SUBSTITUTION = re.compile(r"\{\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}\}")


def template_contract(text: str) -> tuple[list[str], set[str]]:
    """Return the front matter's required_variables and the tokens the body uses."""
    if not text.startswith("---\n"):
        raise AssertionError("template has no front matter")
    front_matter, separator, body = text[len("---\n"):].partition("\n---\n")
    if not separator:
        raise AssertionError("template front matter is not closed by ---")
    required: list[str] = []
    in_required = False
    for line in front_matter.splitlines():
        if line.startswith("required_variables:"):
            in_required = True
        elif in_required and line.startswith("  - "):
            required.append(line[len("  - "):].strip())
        elif in_required and line.strip():
            in_required = False
    if not required:
        raise AssertionError("template front matter declares no required_variables")
    tokens: set[str] = set()
    for expression in TEMPLATE_EXPRESSION.findall(body):
        if expression.startswith("{#"):
            continue
        substitution = PLAIN_SUBSTITUTION.fullmatch(expression)
        if substitution is None:
            raise AssertionError(f"unsupported template expression {expression!r}; use a plain {{{{ name }}}}")
        tokens.add(substitution.group(1))
    return required, tokens


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
    assert "optional `test` and `justfile` fields" in text
    assert "`justfile.windows_shell` is a boolean that defaults to `true`" in text
    for collision in ("`lint`", "`default`", "`set`", "windows_shell", "templates/test.just"):
        assert collision in text
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
        '"justfile": {"windows_shell": true}',
        "The optional\n`justfile` object accepts `windows_shell`, a boolean that defaults to `true`",
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


def test_assignment_template_front_matter_declares_exactly_its_tokens() -> None:
    required, tokens = template_contract(ASSIGNMENT_TEMPLATE.read_text())
    assert len(required) == len(set(required)), f"duplicate required_variables: {required}"
    assert set(required) == tokens


@pytest.mark.parametrize(
    ("text", "message"),
    [
        ("<a>{{ repository }}</a>\n", "no front matter"),
        ("---\nname: x\nrequired_variables:\n  - repository\n<a>{{ repository }}</a>\n", "not closed"),
        ("---\nname: x\n---\n<a>{{ repository }}</a>\n", "no required_variables"),
        ("---\nrequired_variables:\n  - repository\n---\n{% if x %}{{ repository }}{% endif %}\n", "unsupported"),
    ],
)
def test_template_contract_rejects_malformed_templates(text: str, message: str) -> None:
    with pytest.raises(AssertionError, match=message):
        template_contract(text)


def test_template_contract_reports_token_drift() -> None:
    required, tokens = template_contract(
        "---\nrequired_variables:\n  - repository\n  - unused\n---\n<a>{{ repository }}{{ extra }}</a>\n"
    )
    assert required == ["repository", "unused"]
    assert tokens == {"repository", "extra"}
