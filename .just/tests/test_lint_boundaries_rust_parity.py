from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from sc_lint.lint_boundaries import RULE_ID
from sc_lint.lint_boundaries import RUST_REPORT_TIMEOUT_SECONDS
from sc_lint.lint_boundaries import run
from sc_lint.lint_boundaries import validate_inventory
from sc_lint.tests.test_lint_boundaries import VALID_BOUNDARY
from sc_lint.tests.test_lint_boundaries import write_fixture


class LintBoundariesRustParityTests(unittest.TestCase):
    def test_issue377_delegate_parity(self) -> None:
        binary = os.environ["SC_LINT_BIN"]
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            write_fixture(repo_root)
            boundary = repo_root / "boundaries" / "sc-lint" / "top-level-cli.toml"
            boundary.write_text(
                VALID_BOUNDARY.replace('facade = "Cli"', 'facade = "not-valid"'),
                encoding="utf-8",
            )
            direct = subprocess.run(
                [
                    binary,
                    "--json",
                    "--root",
                    str(repo_root),
                    "lint",
                    "sc-boundary",
                    "--rule-filter",
                    RULE_ID,
                ],
                capture_output=True,
                check=False,
                text=True,
                timeout=RUST_REPORT_TIMEOUT_SECONDS,
            )
            self.assertEqual(direct.returncode, 10, direct.stderr)
            report = json.loads(direct.stdout)["data"]
            direct_messages = [finding["message"] for finding in report["findings"]]

            self.assertEqual(validate_inventory(repo_root), direct_messages)
            self.assertEqual(run(repo_root), 1)
            self.assertEqual(report["status"], "fail")


if __name__ == "__main__":
    unittest.main()
