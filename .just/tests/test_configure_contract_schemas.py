from __future__ import annotations

import json
from pathlib import Path
import re
import tomllib
import unittest

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[2]
SCHEMAS = ROOT / "schemas"
FIXTURES = ROOT / "tests" / "fixtures" / "configure" / "contracts"
SPRINT = ROOT / "docs" / "plans" / "phase-F" / "sprint-F1.md"


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def validate(schema_name: str, instance: object) -> None:
    validator = Draft202012Validator(load_json(SCHEMAS / schema_name))
    errors = sorted(validator.iter_errors(instance), key=lambda error: list(error.path))
    if errors:
        rendered = "; ".join(error.message for error in errors)
        raise AssertionError(f"{schema_name} rejected fixture: {rendered}")


def documented_fixture(name: str, language: str = "json") -> object:
    source = SPRINT.read_text(encoding="utf-8")
    match = re.search(
        rf"<!-- configure-contract-fixture: {re.escape(name)} -->\n```{language}\n(.*?)\n```",
        source,
        flags=re.DOTALL,
    )
    if match is None:
        raise AssertionError(f"missing documented fixture marker for {name}")
    if language == "json":
        return json.loads(match.group(1))
    return tomllib.loads(match.group(1))


class ConfigureContractSchemaTests(unittest.TestCase):
    def test_schema_documents_are_valid_draft_2020_12(self) -> None:
        for schema_path in sorted(SCHEMAS.glob("sc-lint-configure-*.schema.json")):
            with self.subTest(schema=schema_path.name):
                Draft202012Validator.check_schema(load_json(schema_path))

    def test_context_request_plan_and_success_fixtures_validate(self) -> None:
        for fixture_name, schema_name in (
            ("context.json", "sc-lint-configure-context.schema.json"),
            ("request.json", "sc-lint-configure-request.schema.json"),
            ("plan.json", "sc-lint-configure-plan.schema.json"),
            ("result-success.json", "sc-lint-configure-result.schema.json"),
        ):
            with self.subTest(fixture=fixture_name):
                validate(schema_name, load_json(FIXTURES / fixture_name))

    def test_every_configure_error_fixture_validates(self) -> None:
        for fixture in sorted(FIXTURES.glob("result-error-*.json")):
            with self.subTest(fixture=fixture.name):
                error_fixture = load_json(fixture)
                validate("sc-lint-configure-result.schema.json", error_fixture)
                self.assertTrue(error_fixture["error"]["message"])
                self.assertTrue(error_fixture["error"]["cause"])
                self.assertTrue(error_fixture["error"]["recovery_description"])

    def test_plan_and_result_reject_invalid_fixtures(self) -> None:
        for fixture_name, schema_name in (
            ("invalid-plan-missing-plan-id.json", "sc-lint-configure-plan.schema.json"),
            ("invalid-result-unknown-error-code.json", "sc-lint-configure-result.schema.json"),
        ):
            with self.subTest(fixture=fixture_name):
                with self.assertRaises(AssertionError):
                    validate(schema_name, load_json(FIXTURES / fixture_name))

    def test_success_data_is_the_verbatim_plan_fixture(self) -> None:
        success = load_json(FIXTURES / "result-success.json")
        self.assertEqual(success["data"], load_json(FIXTURES / "plan.json"))
        validate("sc-lint-configure-plan.schema.json", success["data"])

    def test_sprint_f1_examples_are_the_golden_fixtures(self) -> None:
        fixture_map = {
            "context": "context.json",
            "request": "request.json",
            "plan": "plan.json",
            "result-success": "result-success.json",
        }
        for documented_name, fixture_name in fixture_map.items():
            with self.subTest(fixture=documented_name):
                self.assertEqual(documented_fixture(documented_name), load_json(FIXTURES / fixture_name))

        documented_errors = documented_fixture("result-errors")
        expected_errors = [
            load_json(FIXTURES / fixture_name)
            for fixture_name in (
                "result-error-unsupported-schema.json",
                "result-error-ui-unavailable.json",
                "result-error-unmanaged-collision.json",
                "result-error-stale-plan.json",
                "result-error-rollback-failed.json",
            )
        ]
        self.assertEqual(documented_errors, expected_errors)

    def test_recommended_profile_fixture_is_the_documented_toml_contract(self) -> None:
        expected = tomllib.loads((FIXTURES / "recommended-profile.toml").read_text(encoding="utf-8"))
        self.assertEqual(documented_fixture("recommended-profile", language="toml"), expected)
        profiles = expected["tool"]["sc-lint"]
        self.assertEqual([profile["name"] for profile in profiles["lint"]], ["fmt", "clippy"])
        self.assertEqual(profiles["lint"][0]["command"], ["cargo", "fmt", "--all", "--check"])
        self.assertEqual(
            profiles["lint"][1]["command"],
            ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
        )
        self.assertEqual(profiles["test"][0]["command"], ["cargo", "test", "--workspace"])

    def test_context_and_family_decisions_reject_unsafe_ambiguity(self) -> None:
        context = load_json(FIXTURES / "context.json")
        context["context"]["justfile"]["inspected"] = True
        with self.assertRaises(AssertionError):
            validate("sc-lint-configure-context.schema.json", context)

        request = load_json(FIXTURES / "request.json")
        request["request"]["lint_families"]["boundary"].pop("settings")
        with self.assertRaises(AssertionError):
            validate("sc-lint-configure-request.schema.json", request)

    def test_new_contract_document_links_resolve(self) -> None:
        for document in (
            ROOT / "docs" / "plans" / "phase-F" / "sprint-F1.md",
            ROOT / "docs" / "sc-lint" / "configure-schemas.md",
            ROOT / "docs" / "sc-lint" / "cli-contract.md",
            ROOT / "docs" / "sc-lint" / "README.md",
            ROOT / "docs" / "sc-lint" / "adr" / "ADR-014-consumer-configuration-automation.md",
            ROOT / "docs" / "architecture.md",
            ROOT / "docs" / "project-plan.md",
            ROOT / "docs" / "sc-lint" / "roadmap.md",
        ):
            for link in re.findall(r"\[[^\]]+\]\(([^)]+)\)", document.read_text(encoding="utf-8")):
                if link.startswith(("http://", "https://", "#")):
                    continue
                target = link.split("#", maxsplit=1)[0]
                self.assertTrue((document.parent / target).exists(), f"{document}: {link}")


if __name__ == "__main__":
    unittest.main()
