from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import unittest


ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
FIXTURES = ROOT / "tests" / "fixtures" / "configure"
AGENT_FIXTURES = FIXTURES / "agent"
WIZARD_FIXTURES = ROOT / "docs" / "sc-lint" / "configure-wizard-fixtures"
BINARY = ROOT / "target" / "debug" / ("sc-lint.exe" if sys.platform == "win32" else "sc-lint")
sys.path.insert(0, str(SCRIPTS))

from sc_lint_configure import collect_context
from sc_lint_configure_schema import validate


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def file_snapshot(root: Path) -> tuple[tuple[str, bytes], ...]:
    return tuple(
        (path.relative_to(root).as_posix(), path.read_bytes())
        for path in sorted(root.rglob("*"))
        if path.is_file()
    )


class AgentJsonAndSkillTests(unittest.TestCase):
    scenarios = (
        (
            "empty-rust",
            "empty-rust-context.json",
            "request-recommended.json",
            [
                ("sc-lint.toml", "propose_create", "recommended_profiles:baseline,boundary,portability,attributes"),
                (".sc-lint/justfile", "propose_create", "managed_consumer_recipes"),
                ("Justfile", "needs_confirmation", "managed_import_requires_confirmation"),
            ],
        ),
        (
            "sc-compose",
            "sc-compose-context.json",
            "request-existing-conflict.json",
            [
                ("sc-lint.toml", "needs_confirmation", "existing_sc_lint_config_not_rewritten"),
                ("Justfile", "needs_confirmation", "existing_integration_uninspected"),
                (".github/workflows/sc-lint.yml", "needs_confirmation", "existing_integration_uninspected"),
            ],
        ),
        (
            "atm-core",
            "atm-core-context.json",
            "request-existing-conflict.json",
            [
                ("sc-lint.toml", "propose_create", "recommended_profiles:baseline,boundary,portability,attributes"),
                ("Justfile", "needs_confirmation", "existing_integration_uninspected"),
                (".github/workflows/sc-lint.yml", "needs_confirmation", "existing_integration_uninspected"),
            ],
        ),
    )

    @classmethod
    def setUpClass(cls) -> None:
        # Windows cannot replace the executable while `sc-lint test` is using
        # it. The aggregate gates build first; keep direct test invocation
        # self-contained when the binary is absent.
        if not BINARY.is_file():
            subprocess.run(["cargo", "build", "-q", "-p", "sc-lint"], cwd=ROOT, check=True)

    def configure(
        self, root: Path, request: Path | str, *, input_text: str | None = None
    ) -> subprocess.CompletedProcess[str]:
        environment = os.environ.copy()
        environment["PATH"] = f"{BINARY.parent}{os.pathsep}{environment.get('PATH', '')}"
        return subprocess.run(
            [
                "sc-lint",
                "configure",
                "--request",
                str(request),
                "--root",
                str(root),
                "--dry-run",
                "--json",
            ],
            cwd=ROOT,
            env=environment,
            input=input_text,
            check=False,
            capture_output=True,
            text=True,
        )

    def test_every_f3a_context_and_page_selection_is_deterministic_and_no_write(self) -> None:
        for name, context_name, request_name, expected_operations in self.scenarios:
            with self.subTest(name=name):
                root = AGENT_FIXTURES / name
                request = WIZARD_FIXTURES / request_name
                self.assertEqual(collect_context(root), load_json(WIZARD_FIXTURES / context_name))
                before = file_snapshot(root)

                first = self.configure(root, request)
                second = self.configure(root, request)

                self.assertEqual(first.returncode, 0, first.stderr)
                self.assertEqual(second.returncode, 0, second.stderr)
                self.assertEqual(first.stdout, second.stdout)
                self.assertEqual(before, file_snapshot(root))

                result = json.loads(first.stdout)
                self.assertEqual(validate("result", result), [])
                self.assertEqual(result["command"], "configure.plan")
                self.assertEqual(
                    [
                        (operation["path"], operation["kind"], operation.get("reason"))
                        for operation in result["data"]["operations"]
                    ],
                    expected_operations,
                )
                self.assertEqual(result["data"]["conflicts"], [])
                self.assertEqual(result["data"]["manual_steps"], [])

    def test_documented_standard_input_example_equals_file_example(self) -> None:
        root = AGENT_FIXTURES / "sc-compose"
        request = WIZARD_FIXTURES / "request-existing-conflict.json"
        before = file_snapshot(root)

        from_file = self.configure(root, request)
        from_stdin = self.configure(root, "-", input_text=request.read_text(encoding="utf-8"))

        self.assertEqual(from_file.returncode, 0, from_file.stderr)
        self.assertEqual(from_stdin.returncode, 0, from_stdin.stderr)
        self.assertEqual(from_stdin.stdout, from_file.stdout)
        self.assertEqual(validate("result", json.loads(from_stdin.stdout)), [])
        self.assertEqual(before, file_snapshot(root))

    def test_invalid_pointer_value_combinations_are_recoverable_and_no_write(self) -> None:
        root = AGENT_FIXTURES / "atm-core"
        request = load_json(WIZARD_FIXTURES / "request-existing-conflict.json")
        invalid_requests = (
            (
                "/request/lint_families/boundary",
                {
                    **request,
                    "request": {
                        **request["request"],
                        "lint_families": {
                            **request["request"]["lint_families"],
                            "boundary": {"state": "enabled", "decision": "modify"},
                        },
                    },
                },
            ),
            (
                "/request/just/mode",
                {
                    **request,
                    "request": {
                        **request["request"],
                        "just": {"mode": "invented_shell_migration"},
                    },
                },
            ),
        )

        for pointer, invalid_request in invalid_requests:
            with self.subTest(pointer=pointer):
                before = file_snapshot(root)
                completed = self.configure(root, "-", input_text=json.dumps(invalid_request))

                self.assertEqual(completed.returncode, 3, completed.stdout)
                self.assertEqual(completed.stdout, "")
                failure = json.loads(completed.stderr)
                self.assertEqual(validate("result", failure), [])
                self.assertEqual(failure["command"], "configure.plan")
                self.assertEqual(failure["error"]["code"], "CLI.CONFIGURE_UNSUPPORTED_SCHEMA")
                self.assertEqual(failure["error"]["pointer"], pointer)
                self.assertEqual(failure["error"]["recovery"], "repair_request_schema")
                self.assertTrue(failure["error"]["recovery_description"])
                self.assertTrue(failure["error"]["docs_ref"])
                self.assertEqual(before, file_snapshot(root))

    def test_skill_is_bounded_and_links_to_the_f1_schema_authorities(self) -> None:
        skill = ROOT / ".claude" / "skills" / "sc-lint-consumer-setup" / "SKILL.md"
        reference = skill.parent / "references" / "agent-json.md"
        skill_text = skill.read_text(encoding="utf-8")
        reference_text = reference.read_text(encoding="utf-8")

        self.assertIn("sc-lint configure --request <request.json|-> --root <consumer-root> --dry-run --json", skill_text)
        self.assertIn("Do not add a wrapper, repository probe", skill_text)
        self.assertIn("must not invoke an apply command", skill_text)
        self.assertIn("--request -", reference_text)
        for schema in (
            ROOT / "schemas" / "sc-lint-configure-request.schema.json",
            ROOT / "schemas" / "sc-lint-configure-result.schema.json",
        ):
            self.assertTrue(schema.is_file(), schema)


if __name__ == "__main__":
    unittest.main()
