from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from lint_boundaries import run
from lint_boundaries import validate_inventory


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

[planned_items."BOUNDARY-ScLintCli.public.facade"]
scheduled_sprint = "A.1a"
tracking_id = "SC-LINT-CLI-003"
expires_when = "sprint_before_current"
"""


class LintBoundariesTests(unittest.TestCase):
    def write_fixture(self, repo_root: Path) -> None:
        (repo_root / "Cargo.toml").write_text(
            '[workspace]\nmembers=["crates/example"]\nresolver="2"\n',
            encoding="utf-8",
        )
        crate_dir = repo_root / "crates" / "example"
        crate_dir.mkdir(parents=True)
        (crate_dir / "Cargo.toml").write_text(
            '[package]\nname="example"\nversion="0.1.0"\nedition="2024"\n',
            encoding="utf-8",
        )
        boundaries_dir = repo_root / "boundaries" / "sc-lint"
        boundaries_dir.mkdir(parents=True)
        (boundaries_dir / "top-level-cli.toml").write_text(VALID_BOUNDARY, encoding="utf-8")
        (repo_root / "boundaries" / "planning.toml").write_text(
            VALID_PLANNING,
            encoding="utf-8",
        )

    def test_validate_inventory_accepts_valid_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.write_fixture(repo_root)
            self.assertEqual(validate_inventory(repo_root), [])

    def test_validate_inventory_rejects_invalid_schema(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.write_fixture(repo_root)
            (repo_root / "boundaries" / "sc-lint" / "top-level-cli.toml").write_text(
                VALID_BOUNDARY + '\nunexpected = "nope"\n',
                encoding="utf-8",
            )
            errors = validate_inventory(repo_root)
            self.assertTrue(any("unexpected" in error for error in errors))

    def test_validate_inventory_rejects_duplicate_boundary_ids(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.write_fixture(repo_root)
            duplicate_dir = repo_root / "boundaries" / "other-owner"
            duplicate_dir.mkdir(parents=True)
            duplicate = VALID_BOUNDARY.replace(
                'owner_package = "sc-lint"',
                'owner_package = "other-owner"',
            ).replace(
                'owner_crate_path = "sc_lint"',
                'owner_crate_path = "other_owner"',
            )
            (duplicate_dir / "duplicate.toml").write_text(duplicate, encoding="utf-8")
            errors = validate_inventory(repo_root)
            self.assertTrue(any("duplicate boundary_id" in error for error in errors))

    @mock.patch("lint_boundaries.print_report")
    @mock.patch("lint_boundaries.build_report")
    def test_run_reports_failure_from_validation_errors(
        self,
        build_report_mock: mock.Mock,
        print_report_mock: mock.Mock,
    ) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            self.write_fixture(repo_root)
            (repo_root / "boundaries" / "planning.toml").write_text(
                "[planning]\ncurrent_sprint = \"\"\n",
                encoding="utf-8",
            )
            build_report_mock.return_value = mock.Mock(
                log_path=repo_root / ".just/logs/example.log"
            )

            self.assertEqual(run(repo_root), 1)
            self.assertTrue(build_report_mock.called)
            print_report_mock.assert_called_once()


if __name__ == "__main__":
    unittest.main()
