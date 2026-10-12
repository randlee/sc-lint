from __future__ import annotations

import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from sc_lint.lint_boundaries import validate_inventory
from sc_lint.tests.test_lint_boundaries import VALID_BOUNDARY
from sc_lint.tests.test_lint_boundaries import write_fixture


class LintBoundariesRustParityTests(unittest.TestCase):
    def test_validate_inventory_matches_rust_for_widened_schema_and_lib_target(self) -> None:
        source_root = Path.cwd()
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            write_fixture(repo_root)
            crate_manifest = repo_root / "crates" / "example" / "Cargo.toml"
            crate_manifest.write_text(
                crate_manifest.read_text(encoding="utf-8") + '\n[lib]\nname = "cli_core"\n',
                encoding="utf-8",
            )
            boundary = repo_root / "boundaries" / "sc-lint" / "top-level-cli.toml"
            boundary.write_text(
                VALID_BOUNDARY.replace('owner_crate_path = "sc_lint"', 'owner_crate_path = "cli_core"')
                .replace("forbidden_edges = []", "allowed_dev_dependencies = []\nforbidden_edges = []")
                + '\n[extensions.example]\nsetting = "opaque"\n',
                encoding="utf-8",
            )

            self.assertEqual(validate_inventory(repo_root), [])
            result = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--manifest-path",
                    str(source_root / "Cargo.toml"),
                    "-p",
                    "sc-lint-boundary",
                    "--",
                    "analyze",
                    "--root",
                    str(repo_root),
                    "--rule",
                    "dependencies",
                    "--format",
                    "json",
                ],
                cwd=source_root,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["status"], "pass")

            boundary.write_text(
                boundary.read_text(encoding="utf-8").replace(
                    'owner_crate_path = "cli_core"', 'owner_crate_path = "wrong_name"'
                ),
                encoding="utf-8",
            )
            self.assertTrue(validate_inventory(repo_root))
            result = subprocess.run(
                [
                    "cargo", "run", "--quiet", "--manifest-path", str(source_root / "Cargo.toml"),
                    "-p", "sc-lint-boundary", "--", "analyze", "--root", str(repo_root),
                    "--rule", "dependencies", "--format", "json",
                ],
                cwd=source_root,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(
                result.returncode,
                0,
                "Rust analyzer accepted a boundary with the wrong owner crate path",
            )
            self.assertIn("declares owner_crate_path `wrong_name`", result.stderr)
            self.assertIn("expected `cli_core`", result.stderr)
