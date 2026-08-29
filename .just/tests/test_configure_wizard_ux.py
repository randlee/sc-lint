from __future__ import annotations

import json
from pathlib import Path
import re
import unittest

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[2]
SCHEMAS = ROOT / "schemas"
FIXTURES = ROOT / "docs" / "sc-lint" / "configure-wizard-fixtures"
UX = ROOT / "docs" / "sc-lint" / "configure-wizard-ux.md"
README = FIXTURES / "README.md"
ADR = ROOT / "docs" / "sc-lint" / "adr" / "ADR-014-consumer-configuration-automation.md"


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def schema(name: str) -> dict:
    return load_json(SCHEMAS / name)


def validate(instance: object, schema_value: dict, label: str) -> None:
    errors = sorted(Draft202012Validator(schema_value).iter_errors(instance), key=str)
    if errors:
        rendered = "; ".join(error.message for error in errors)
        raise AssertionError(f"{label} is invalid: {rendered}")


class ConfigureWizardUxTests(unittest.TestCase):
    def test_fixture_contracts_and_provenance_are_valid(self) -> None:
        context_schema = schema("sc-lint-configure-context.schema.json")
        request_schema = schema("sc-lint-configure-request.schema.json")
        plan_schema = schema("sc-lint-configure-plan.schema.json")
        context_fact_schema = {
            "$defs": context_schema["$defs"],
            "$ref": "#/$defs/context",
        }

        for name in ("empty-rust-context.json", "sc-compose-context.json", "atm-core-context.json"):
            with self.subTest(fixture=name):
                fixture = load_json(FIXTURES / name)
                self.assertEqual(set(fixture), {"schema_version", "context", "source"})
                self.assertEqual(fixture["schema_version"], "v1")
                validate(fixture["context"], context_fact_schema, name)
                source = fixture["source"]
                self.assertEqual(set(source), {"repository", "repository_url", "baseline_commit"})
                self.assertTrue(source["repository"])
                self.assertTrue(source["repository_url"].startswith("https://github.com/"))
                self.assertRegex(source["baseline_commit"], r"^[0-9a-f]{40}$")

        for name in ("request-recommended.json", "request-existing-conflict.json"):
            with self.subTest(fixture=name):
                validate(load_json(FIXTURES / name), request_schema, name)

        validate(load_json(FIXTURES / "plan-no-write-conflict.json"), plan_schema, "conflict plan")

    def test_empty_rust_fixture_reproduces_f2_context_facts(self) -> None:
        import sys

        sys.path.insert(0, str(ROOT / "scripts"))
        from sc_lint_configure import collect_context

        generated = collect_context(ROOT / "tests" / "fixtures" / "configure" / "empty-rust")
        fixture = load_json(FIXTURES / "empty-rust-context.json")
        self.assertEqual(generated["context"], fixture["context"])

    def test_fixture_json_contains_no_sensitive_or_local_content(self) -> None:
        forbidden = ("/Users/", "\\\\Users\\\\", "~/", "password", "authorization", "token")
        for fixture in sorted(FIXTURES.glob("*.json")):
            with self.subTest(fixture=fixture.name):
                text = fixture.read_text(encoding="utf-8").lower()
                self.assertFalse(any(value.lower() in text for value in forbidden))
                self.assertNotIn("#!", text)

    def test_page_contract_is_complete_and_links_resolve(self) -> None:
        source = UX.read_text(encoding="utf-8")
        required = (
            "Overview",
            "Baseline",
            "Boundary",
            "Portability",
            "Runtime",
            "Attributes/directives",
            "Command groups",
            "Just integration",
            "CI integration",
            "Final review",
        )
        for index, title in enumerate(required, start=1):
            with self.subTest(page=title):
                marker = f"## {index}. {title}"
                self.assertIn(marker, source)
                start = source.index(marker)
                next_marker = f"## {index + 1}." if index < len(required) else "## Terminal"
                end = source.find(next_marker, start + len(marker))
                section = source[start : None if end == -1 else end]
                self.assertIn("| Visible field | Pointer |", section)
                self.assertIn("**Footer:**", section)

        for pointer in (
            "/explanation/developer_contract",
            "/request/minimum_version",
            "/request/lint_families/baseline",
            "/request/lint_families/boundary",
            "/request/lint_families/portability",
            "/request/lint_families/runtime",
            "/request/lint_families/attributes",
            "/request/consumer_command_groups",
            "/request/just/mode",
            "/request/ci/mode",
            "/operations",
        ):
            self.assertIn(pointer, source)

        for document in (UX, README, ADR):
            for target in re.findall(r"\[[^\]]+\]\(([^)]+)\)", document.read_text(encoding="utf-8")):
                if target.startswith(("http://", "https://", "#")):
                    continue
                self.assertTrue(
                    (document.parent / target.split("#", maxsplit=1)[0]).exists(),
                    f"{document}: {target}",
                )

    def test_capability_gate_and_adr_are_explicit(self) -> None:
        source = UX.read_text(encoding="utf-8")
        for capability in (
            "Multi-page descriptors",
            "Browser-history restoration",
            "Conditional next-page branching",
            "Opaque per-page data",
            "Cancel and dismiss",
            "Finish with full stack",
            "Local-only serving",
            "Deterministic headless tests",
        ):
            self.assertIn(capability, source)
        self.assertIn("capability-gated", ADR.read_text(encoding="utf-8"))
        self.assertIn("configure-wizard-ux.md", ADR.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
