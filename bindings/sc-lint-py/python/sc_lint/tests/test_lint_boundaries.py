from __future__ import annotations

import ast
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

from sc_lint import lint_boundaries
from sc_lint.lint_boundaries import RULE_ID
from sc_lint.lint_boundaries import RUST_REPORT_TIMEOUT_SECONDS
from sc_lint.lint_boundaries import delegation_argv
from sc_lint.lint_boundaries import run
from sc_lint.lint_boundaries import rust_report
from sc_lint.lint_boundaries import validate_inventory


VALID_BOUNDARY = """
boundary_id = "BOUNDARY-ScLintCli"
owner_package = "sc-lint"
owner_crate_path = "sc_lint"
name = "ScLintCli"

[public]
facade = "Cli"

[implementation]
type = "Cli"
module = "sc_lint"
visibility = "public"
constructor = "none"

[composition]
roots = ["Cli"]

[dependencies]
allowed_dependents = []
allowed_dependencies = []
forbidden_edges = []

[references]
scope = "outside_owner_crate"
forbidden = []

[testing]
allowed_test_double_paths = []
forbidden_test_bypasses = []

[enforcement]
lint_rules = []
review_gates = []

[status]
state = "concrete_landed"
"""

VALID_PLANNING = """
[planning]
current_sprint = "A.6"
"""


def write_fixture(repo_root: Path) -> None:
    (repo_root / "Cargo.toml").write_text(
        '[workspace]\nmembers=["crates/example"]\nresolver="2"\n', encoding="utf-8"
    )
    crate_dir = repo_root / "crates" / "example"
    crate_dir.mkdir(parents=True)
    (crate_dir / "Cargo.toml").write_text(
        '[package]\nname="sc-lint"\nversion="0.1.0"\nedition="2024"\n',
        encoding="utf-8",
    )
    (crate_dir / "src").mkdir()
    (crate_dir / "src" / "lib.rs").write_text("pub struct Cli;\n", encoding="utf-8")
    boundaries_dir = repo_root / "boundaries" / "sc-lint"
    boundaries_dir.mkdir(parents=True)
    (boundaries_dir / "top-level-cli.toml").write_text(VALID_BOUNDARY, encoding="utf-8")
    (repo_root / "boundaries" / "planning.toml").write_text(VALID_PLANNING, encoding="utf-8")


class LintBoundariesTests(unittest.TestCase):
    def test_delegation_argv_uses_only_the_rust_rule(self) -> None:
        root = Path("/tmp/example")
        with mock.patch("sc_lint.lint_boundaries.binary_path", return_value="/bin/sc-lint"):
            self.assertEqual(
                delegation_argv(root),
                [
                    "/bin/sc-lint",
                    "--json",
                    "--root",
                    str(root),
                    "lint",
                    "sc-boundary",
                    "--rule-filter",
                    RULE_ID,
                ],
            )

    def test_run_projects_rust_status(self) -> None:
        payload = json.dumps({"data": {"status": "fail", "findings": []}})
        completed = mock.Mock(returncode=0, stdout=payload, stderr="")
        with mock.patch("sc_lint.lint_boundaries.subprocess.run", return_value=completed):
            self.assertEqual(run(Path("/tmp/example")), 1)

    def test_validate_inventory_projects_rust_messages(self) -> None:
        payload = json.dumps(
            {"data": {"status": "fail", "findings": [{"message": "Rust owns this"}]}}
        )
        completed = mock.Mock(returncode=0, stdout=payload, stderr="")
        with mock.patch("sc_lint.lint_boundaries.subprocess.run", return_value=completed):
            self.assertEqual(validate_inventory(Path("/tmp/example")), ["Rust owns this"])

    def test_rust_report_bounds_delegation_with_named_timeout(self) -> None:
        argv = ["/bin/sc-lint", "--json", "lint", "sc-boundary"]
        expired = subprocess.TimeoutExpired(cmd=argv, timeout=RUST_REPORT_TIMEOUT_SECONDS)
        with mock.patch("sc_lint.lint_boundaries.binary_path", return_value="/bin/sc-lint"):
            with mock.patch(
                "sc_lint.lint_boundaries.subprocess.run", side_effect=expired
            ) as run_mock:
                with self.assertRaises(RuntimeError) as error:
                    rust_report(Path("/tmp/example"))
        self.assertEqual(run_mock.call_args.kwargs["timeout"], RUST_REPORT_TIMEOUT_SECONDS)
        self.assertIn(f"timed out after {RUST_REPORT_TIMEOUT_SECONDS}s", str(error.exception))
        self.assertIn("/bin/sc-lint", str(error.exception))
        self.assertIn(RULE_ID, str(error.exception))

    def test_issue377_delegate_parity(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            write_fixture(repo_root)
            boundary = repo_root / "boundaries" / "sc-lint" / "top-level-cli.toml"
            boundary.write_text(
                VALID_BOUNDARY.replace('facade = "Cli"', 'facade = "not-valid"'),
                encoding="utf-8",
            )
            messages = validate_inventory(repo_root)
            self.assertEqual(len(messages), 1)
            self.assertIn("invalid Rust identifier `not-valid`", messages[0])

    def test_module_exposes_only_the_delegation_surface(self) -> None:
        tree = ast.parse(Path(lint_boundaries.__file__).read_text(encoding="utf-8"))
        defined: set[str] = set()
        for node in tree.body:
            if isinstance(node, (ast.FunctionDef, ast.ClassDef)):
                defined.add(node.name)
            elif isinstance(node, ast.Assign):
                defined.update(t.id for t in node.targets if isinstance(t, ast.Name))
            elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
                defined.add(node.target.id)
        self.assertEqual(
            defined,
            {"RULE_ID", "delegation_argv", "rust_report", "validate_inventory", "run", "main"},
        )


if __name__ == "__main__":
    unittest.main()
