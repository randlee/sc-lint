"""Regression tests for repository-local orchestration helpers and templates.

These tests deliberately import scripts by path and inject subprocess results;
they never require a live ATM or Herdr daemon.
"""

from __future__ import annotations

import importlib.util
import json
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
    TEMPLATE_DIR = REPO / ".claude/skills/codex-orchestration"
    EXPECTED_JSON_KEYS = {
        "arch-qa-assignment.json.j2": {"authoritative_sprint_doc", "branch", "carry_forward_findings", "changed_files", "commit", "notes", "reference_docs", "review_mode", "review_targets", "round_limit", "scope", "triage_records", "worktree_path"},
        "flaky-test-qa-assignment.json.j2": {"carry_forward_findings", "changed_files", "notes", "review_targets", "round_limit", "scope", "triage_records", "worktree_path"},
        "req-qa-assignment.json.j2": {"authoritative_sprint_doc", "branch", "carry_forward_findings", "changed_files", "commit", "notes", "phase_or_sprint_docs", "phase_sprint_documents", "review_targets", "round_limit", "scope", "triage_records", "worktree_path"},
        "ruthless-boundary-qa-assignment.json.j2": {"review_mode", "worktree_path", "review_targets", "reference_docs", "round_limit", "changed_files", "duplicate_sweep_symbols", "triage_records", "carry_forward_findings", "findings_scope_locked", "notes"},
    }

    def test_every_orchestration_template_has_a_sample_and_composes(self) -> None:
        if shutil.which("atm") is None:
            self.skipTest("atm is not on PATH; cannot exercise daemon template composition")
        for template in sorted(self.TEMPLATE_DIR.glob("*.j2")):
            with self.subTest(template=template.name):
                sample_name = template.name.removesuffix(".j2")
                if not sample_name.endswith(".json"):
                    sample_name += ".json"
                sample = self.TEMPLATE_DIR / "vars" / sample_name
                self.assertTrue(sample.is_file(), f"missing committed sample vars: {sample}")
                result = subprocess.run(["atm", "compose", "--template", str(template), "--vars", str(sample)], capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                if template.name in self.EXPECTED_JSON_KEYS:
                    payload = json.loads(result.stdout)
                    self.assertEqual(set(payload), self.EXPECTED_JSON_KEYS[template.name])

    def test_missing_sample_var_fails_composition(self) -> None:
        if shutil.which("atm") is None:
            self.skipTest("atm is not on PATH; cannot exercise daemon template composition")
        template = self.TEMPLATE_DIR / "ruthless-boundary-qa-assignment.json.j2"
        with tempfile.TemporaryDirectory() as directory:
            bad_vars = Path(directory) / "bad.json"
            bad_vars.write_text('{"review_mode":"sprint","worktree_path":"/tmp"}', encoding="utf-8")
            result = subprocess.run(["atm", "compose", "--template", str(template), "--vars", str(bad_vars)], capture_output=True, text=True, check=False)
        self.assertNotEqual(result.returncode, 0)
