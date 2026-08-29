from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
FIXTURES = ROOT / "tests" / "fixtures" / "configure"
CONTRACTS = FIXTURES / "contracts"
sys.path.insert(0, str(SCRIPTS))

from sc_lint_configure import ConfigureFailure
from sc_lint_configure import build_plan
from sc_lint_configure import collect_context
from sc_lint_configure import load_request
from sc_lint_configure import plan_result
from sc_lint_configure import render_failure
from sc_lint_configure_schema import validate


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


class ContextAndPlanTests(unittest.TestCase):
    def setUp(self) -> None:
        self.request = load_json(CONTRACTS / "request.json")

    def test_empty_rust_workspace_has_bounded_context_and_recommendations(self) -> None:
        context = collect_context(FIXTURES / "empty-rust")
        self.assertEqual(context["context"]["cargo_toml"], {"present": True, "kind": "workspace"})
        self.assertEqual(context["explanation"]["uninspected_existing_integration"], [])

        plan = build_plan(context, self.request)
        self.assertEqual(plan["operations"][0]["path"], "sc-lint.toml")
        self.assertEqual(
            plan["operations"][0]["reason"],
            "recommended_profiles:baseline,boundary,portability,attributes",
        )
        self.assertTrue(any(operation["path"] == ".sc-lint/justfile" for operation in plan["operations"]))

    def test_existing_just_and_workflow_are_visible_but_uninspected(self) -> None:
        context = collect_context(FIXTURES / "existing-just")
        self.assertEqual(context["context"]["justfile"], {"present": True, "inspected": False})
        self.assertEqual(
            context["context"]["github_workflows"], {"present": True, "inspected": False}
        )
        self.assertEqual(
            context["explanation"]["uninspected_existing_integration"],
            ["Justfile", ".github/workflows/"],
        )

        plan = build_plan(context, self.request)
        uninspected = [
            operation
            for operation in plan["operations"]
            if operation["reason"] == "existing_integration_not_inspected"
        ]
        self.assertEqual([operation["path"] for operation in uninspected], ["Justfile", ".github/workflows/sc-lint.yml"])
        self.assertNotIn("manual_conflict", [operation["kind"] for operation in plan["operations"]])

    def test_existing_workflow_fixture_is_uninspected_without_a_justfile(self) -> None:
        context = collect_context(FIXTURES / "existing-workflow")
        self.assertEqual(context["context"]["justfile"], {"present": False})
        self.assertEqual(
            context["context"]["github_workflows"], {"present": True, "inspected": False}
        )

    def test_ambiguous_cargo_file_is_not_claimed_as_a_rust_package_or_workspace(self) -> None:
        context = collect_context(FIXTURES / "unknown-existing")
        self.assertEqual(context["context"]["cargo_toml"], {"present": False})

    def test_plan_json_is_deterministic_and_validates_against_f1_contracts(self) -> None:
        root = FIXTURES / "empty-rust"
        first = plan_result(root, self.request)
        second = plan_result(root, self.request)
        self.assertEqual(
            json.dumps(first, sort_keys=True, separators=(",", ":")),
            json.dumps(second, sort_keys=True, separators=(",", ":")),
        )
        self.assertEqual(validate("plan", first["data"]), [])
        self.assertEqual(validate("result", first), [])

    def test_invalid_request_uses_f1_validation_and_stable_recovery_envelope(self) -> None:
        invalid_request = json.loads(json.dumps(self.request))
        invalid_request["request"]["consumer_profiles"] = [
            {"kind": "lint", "name": "invalid", "command": [""]}
        ]
        with self.assertRaises(ConfigureFailure) as raised:
            build_plan(collect_context(FIXTURES / "empty-rust"), invalid_request)
        failure = render_failure(raised.exception)
        self.assertEqual(failure["error"]["code"], "CLI.CONFIGURE_UNSUPPORTED_SCHEMA")
        self.assertEqual(failure["error"]["pointer"], "/request/consumer_profiles/0/command/0")
        self.assertEqual(validate("result", failure), [])

    def test_unknown_schema_and_invalid_family_settings_are_structured_failures(self) -> None:
        unsupported_schema = json.loads(json.dumps(self.request))
        unsupported_schema["schema_version"] = "v999"
        invalid_family = json.loads(json.dumps(self.request))
        invalid_family["request"]["boundary"] = {"state": "enabled", "decision": "modify"}
        invalid_family["request"]["lint_families"]["boundary"] = invalid_family["request"].pop("boundary")

        for invalid_request, pointer in (
            (unsupported_schema, "/schema_version"),
            (invalid_family, "/request/lint_families/boundary"),
        ):
            with self.subTest(pointer=pointer), self.assertRaises(ConfigureFailure) as raised:
                build_plan(collect_context(FIXTURES / "empty-rust"), invalid_request)
            self.assertEqual(render_failure(raised.exception)["error"]["pointer"], pointer)

    def test_missing_root_and_malformed_request_are_structured_failures(self) -> None:
        with self.assertRaises(ConfigureFailure) as missing_root:
            collect_context(FIXTURES / "does-not-exist")
        self.assertEqual(render_failure(missing_root.exception)["error"]["pointer"], "/root")

        with tempfile.TemporaryDirectory() as temporary_directory:
            malformed = Path(temporary_directory) / "request.json"
            malformed.write_text("{", encoding="utf-8")
            with self.assertRaises(ConfigureFailure) as malformed_request:
                load_request(str(malformed))
        self.assertEqual(render_failure(malformed_request.exception)["error"]["pointer"], "/request")

    def test_unreadable_target_is_a_structured_failure(self) -> None:
        with patch("sc_lint_configure.os.access", return_value=False), self.assertRaises(
            ConfigureFailure
        ) as unreadable:
            collect_context(FIXTURES / "empty-rust")
        failure = render_failure(unreadable.exception)
        self.assertEqual(failure["error"]["pointer"], "/root")
        self.assertEqual(failure["error"]["recovery"], "repair_root_permissions")

    def test_context_and_planning_start_no_child_process(self) -> None:
        with patch.object(subprocess, "Popen") as process:
            plan_result(FIXTURES / "empty-rust", self.request)
        process.assert_not_called()


if __name__ == "__main__":
    unittest.main()
