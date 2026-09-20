"""Regression tests for repository-local orchestration helpers and templates.

These tests deliberately import scripts by path and inject subprocess results;
they never require a live ATM or Herdr daemon.
"""

from __future__ import annotations

import importlib.util
import json
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from sc_lint.lint_common import discover_repo_root

REPO = discover_repo_root()


def load(name: str, relative: str):
    spec = importlib.util.spec_from_file_location(name, REPO / relative)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class DependencyContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.module = load("sc_compose_dependency_test", ".claude/lib/sc_compose_dependency.py")
        self.check = load(
            "check_dependencies_test",
            ".claude/skills/triaging-findings/scripts/check_dependencies.py",
        )

    def test_version_parser_handles_missing_prerelease_and_extra_text(self) -> None:
        self.assertIsNone(self.module.parse_version(None))
        self.assertIsNone(self.module.parse_version("missing"))
        self.assertEqual(self.module.parse_version("sc-compose 1.6.1-rc.1"), (1, 6, 1))
        self.assertEqual(self.module.parse_version("tool v2.10.3 built today"), (2, 10, 3))

    def test_missing_optional_cli_is_a_structured_failure(self) -> None:
        with mock.patch.object(self.check, "_find", return_value=None):
            result = self.check.run()
        self.assertFalse(result["success"])
        self.assertEqual(result["error"]["code"], "EXECUTION.DEPENDENCY")

    def test_nonzero_version_exit_is_reported(self) -> None:
        with mock.patch.object(
            self.check.subprocess,
            "run",
            return_value=subprocess.CompletedProcess(["tool"], 7, "", "broken version"),
        ):
            version, detail = self.check._run_version(Path("/tool"))
        self.assertIsNone(version)
        self.assertEqual(detail, "broken version")


class ExternalCliPolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.module = load("orchestration_test_cli", ".claude/lib/orchestration_test_cli.py")

    def test_present_cli_returns(self) -> None:
        with mock.patch.object(self.module.shutil, "which", return_value="/tool"):
            self.module.require_dev_cli("tool", ">= 1", "install tool")

    def test_missing_cli_skips_in_ci(self) -> None:
        with mock.patch.object(self.module.shutil, "which", return_value=None), \
             mock.patch.object(self.module.os, "getenv", return_value="1"), \
             mock.patch.object(self.module.pytest, "skip") as skip:
            self.module.require_dev_cli("tool", ">= 1", "install tool")
        skip.assert_called_once_with("dev-host-only check: tool not installed in CI")

    def test_missing_cli_fails_on_development_host(self) -> None:
        with mock.patch.object(self.module.shutil, "which", return_value=None), \
             mock.patch.object(self.module.os, "getenv", return_value=None), \
             mock.patch.object(self.module.pytest, "fail") as fail:
            self.module.require_dev_cli("tool", ">= 1", "install tool")
        fail.assert_called_once_with("tool CLI is required (>= 1); install it with: install tool")


class RosterCheckTests(unittest.TestCase):
    def setUp(self) -> None:
        self.module = load("roster_check_test", ".claude/skills/team-lead/scripts/roster_check.py")
        self.config = {"team-lead": "lead-pane", "quality-mgr": "qa-pane", "publisher": "pub-pane", "clint": None}
        self.roster = [
            {"name": "team-lead", "alias": "lead-pane", "backend": "herdr"},
            {"name": "quality-mgr", "alias": "qa-pane", "backend": "herdr"},
            {"name": "publisher", "alias": "pub-pane", "backend": "herdr"},
            {"name": "clint", "alias": None, "backend": "herdr"},
        ]

    def test_alias_and_unnamed_or_mismatched_herdr_agents_are_detected(self) -> None:
        missing_alias = self.module.find_problems(self.config, self.roster, ["lead-pane", "qa-pane", "clint"])
        self.assertIn("publisher: no live Herdr agent named 'pub-pane'", missing_alias)
        unnamed = self.module.find_problems(self.config, self.roster, ["lead-pane", "qa-pane", "pub-pane"])
        self.assertIn("clint: no live Herdr agent named 'clint'", unnamed)
        mismatch = self.module.find_problems(self.config, self.roster, ["lead-pane", "qa-pane", "pub-pane", "wrong-clint"])
        self.assertIn("clint: no live Herdr agent named 'clint'", mismatch)

    def test_missing_pane_and_nonzero_command_are_reported_without_daemons(self) -> None:
        problems = self.module.find_problems({k: v for k, v in self.config.items() if k != "clint"}, self.roster, ["lead-pane", "qa-pane", "pub-pane", "clint"])
        self.assertIn("clint: in the roster but has no pane in .atm.toml", problems)
        with mock.patch.object(self.module.subprocess, "run", return_value=subprocess.CompletedProcess(["atm"], 1, "", "offline")):
            with self.assertRaisesRegex(RuntimeError, "offline"):
                self.module.run_json(["atm", "members"])


class FindingScriptTests(unittest.TestCase):
    def test_validator_rejects_missing_and_malformed_inputs(self) -> None:
        try:
            validator = load("validate_findings_test", ".claude/skills/graph-orchestration/scripts/validate-findings.py")
        except (ModuleNotFoundError, SystemExit) as error:
            self.skipTest(str(error))
        if getattr(validator, "_RDFLIB_ERROR", None):
            self.skipTest(f"rdflib unavailable: {validator._RDFLIB_ERROR}")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            missing = validator.run_validation(findings_dir=root / "missing")
            self.assertEqual(missing.kind, "error")
            findings = root / "findings"
            findings.mkdir()
            (findings / "broken.ttl").write_text("not Turtle [", encoding="utf-8")
            malformed = validator.run_validation(findings_dir=findings)
        self.assertEqual(malformed.kind, "error")
        self.assertIn("malformed Turtle", malformed.diagnostics[0])

    def test_query_selection_handles_empty_and_unknown_sprint(self) -> None:
        try:
            query = load("query_open_findings_test", ".claude/skills/closing-triage/scripts/query_open_findings.py")
        except (ModuleNotFoundError, SystemExit) as error:
            self.skipTest(str(error))
        if getattr(query, "_RDFLIB_ERROR", None):
            self.skipTest(f"rdflib unavailable: {query._RDFLIB_ERROR}")
        self.assertIsNone(query.choose_integration_candidate([], "SPX"))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            phase_dir = root / ".sprints" / "SPX"
            phase_dir.mkdir(parents=True)
            (phase_dir / "structure.ttl").write_text(
                "@prefix triage: <urn:atm:triage:> .\ntriage:S1 a triage:Sprint ; triage:inPhase triage:SPX ; triage:branch \"feature/s1\" .\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(query.QueryError, "sprint 'S9'"):
                query._sprint_for_branch(root, "fix/s9", "SPX", "S9")


class TemplateContractTests(unittest.TestCase):
    TEMPLATE_DIRS = (
        REPO / ".claude/skills/codex-orchestration",
        REPO / ".claude/assets/sc-rust/quality-mgr/templates",
    )
    AGENT_CONTRACTS = {
        "arch-qa-assignment.json.j2": REPO / ".claude/agents/arch-qa.md",
        "flaky-test-qa-assignment.json.j2": REPO / ".claude/agents/flaky-test-qa.md",
        "req-qa-assignment.json.j2": REPO / ".claude/agents/req-qa.md",
        "ruthless-boundary-qa-assignment.json.j2": REPO / ".claude/agents/ruthless-boundary-qa.md",
        "rust-best-practices-assignment.json.j2": REPO / ".claude/agents/rust-best-practices-agent.md",
        "rust-qa-assignment.json.j2": REPO / ".claude/agents/rust-qa-agent.md",
        "rust-service-hardening-assignment.json.j2": REPO / ".claude/agents/rust-service-hardening-agent.md",
    }

    @staticmethod
    def fenced_json_keys(agent: Path) -> set[str]:
        match = re.search(r"```json\s*\n(.*?)\n```", agent.read_text(encoding="utf-8"), re.S)
        if match is None:
            raise AssertionError(f"missing fenced JSON input contract: {agent}")
        return set(re.findall(r'^  "([^"]+)"\s*:', match.group(1), re.M))

    @staticmethod
    def jinja_parts(template: Path) -> tuple[dict[str, object], str]:
        """Return ATM defaults and body for the Jinja CI-only fallback."""
        content = template.read_text(encoding="utf-8")
        if content.startswith("---\n"):
            _, front_matter, content = content.split("---\n", 2)
            import yaml

            metadata = yaml.safe_load(front_matter)
            return metadata.get("defaults", {}), content
        return {}, content

    def compose_with_jinja(self, template: Path, sample: Path) -> str:
        from jinja2 import Environment, StrictUndefined
        from markupsafe import Markup

        defaults, body = self.jinja_parts(template)
        variables = {**defaults, **json.loads(sample.read_text(encoding="utf-8"))}
        renderer = Environment(
            autoescape=False,
            undefined=StrictUndefined,
            finalize=lambda value: value if isinstance(value, Markup) else json.dumps(value),
        )
        return renderer.from_string(body).render(**variables)

    def compose(self, template: Path, sample: Path) -> str:
        """Use ATM when available; otherwise render JSON-compatible Jinja."""
        if shutil.which("atm"):
            result = subprocess.run(
                ["atm", "compose", "--template", str(template), "--vars", str(sample)],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            return result.stdout

        return self.compose_with_jinja(template, sample)

    def assert_json_contract(self, template: Path, rendered: str) -> None:
        agent = self.AGENT_CONTRACTS.get(template.name)
        self.assertIsNotNone(agent, f"JSON template has no receiving-agent mapping: {template}")
        payload = json.loads(rendered)
        self.assertEqual(set(payload), self.fenced_json_keys(agent))

    def templates(self) -> list[Path]:
        return [
            template
            for directory in self.TEMPLATE_DIRS
            for template in sorted(directory.glob("*.j2"))
        ]

    def test_every_orchestration_template_has_a_sample_and_composes(self) -> None:
        templates = self.templates()
        json_templates = [template for template in templates if template.name.endswith(".json.j2")]
        self.assertEqual({template.name for template in json_templates}, set(self.AGENT_CONTRACTS))
        compared = 0
        for template in templates:
            with self.subTest(template=template.name):
                sample_name = template.name.removesuffix(".j2")
                if not sample_name.endswith(".json"):
                    sample_name += ".json"
                sample = template.parent / "vars" / sample_name
                self.assertTrue(sample.is_file(), f"missing committed sample vars: {sample}")
                rendered = self.compose(template, sample)
                if template.name.endswith(".json.j2"):
                    self.assert_json_contract(template, rendered)
                    compared += 1
        self.assertGreater(compared, 0)
        print(f"compared JSON template contracts: {compared}")

    def test_in_memory_json_template_rejects_extra_or_missing_agent_key(self) -> None:
        from jinja2 import Environment, StrictUndefined

        template = self.TEMPLATE_DIRS[0] / "flaky-test-qa-assignment.json.j2"
        expected = self.fenced_json_keys(self.AGENT_CONTRACTS[template.name])
        renderer = Environment(autoescape=False, undefined=StrictUndefined)
        for bad_keys in (expected - {"notes"}, expected | {"unexpected"}):
            with self.subTest(keys=bad_keys):
                rendered = renderer.from_string(json.dumps({key: None for key in bad_keys})).render()
                with self.assertRaises(AssertionError):
                    self.assert_json_contract(template, rendered)

    def test_missing_sample_var_fails_composition(self) -> None:
        template = self.TEMPLATE_DIRS[0] / "ruthless-boundary-qa-assignment.json.j2"
        with tempfile.TemporaryDirectory() as directory:
            bad_vars = Path(directory) / "bad.json"
            bad_vars.write_text('{"review_mode":"sprint","worktree_path":"/tmp"}', encoding="utf-8")
            if shutil.which("atm"):
                result = subprocess.run(["atm", "compose", "--template", str(template), "--vars", str(bad_vars)], capture_output=True, text=True, check=False)
                self.assertNotEqual(result.returncode, 0)
            else:
                with self.assertRaisesRegex(Exception, "undefined|Undefined"):
                    self.compose(template, bad_vars)

    def test_carry_forward_findings_render_as_json_arrays(self) -> None:
        expected = [{"id": "F-1", "severity": "important"}]
        for template_name in self.AGENT_CONTRACTS:
            template = next(
                directory / template_name
                for directory in self.TEMPLATE_DIRS
                if (directory / template_name).is_file()
            )
            sample_name = template.name.removesuffix(".j2")
            sample = template.parent / "vars" / sample_name
            variables = json.loads(sample.read_text(encoding="utf-8"))
            variables["carry_forward_findings_json"] = json.dumps(expected)
            with self.subTest(template=template.name), tempfile.TemporaryDirectory() as directory:
                vars_path = Path(directory) / "vars.json"
                vars_path.write_text(json.dumps(variables), encoding="utf-8")
                payload = json.loads(self.compose(template, vars_path))
                self.assertEqual(payload["carry_forward_findings"], expected)
