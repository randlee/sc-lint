from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
MANIFEST_ADAPTER = REPO_ROOT / "bindings/sc-lint-py/python/sc_lint/lint_manifests.py"


class PhaseKAnalyzerDelegationTests(unittest.TestCase):
    def test_issue136_native_run_parity(self) -> None:
        argv = [
            "sc-lint",
            "--json",
            "--root",
            str(REPO_ROOT),
            "lint",
            "sc-boundary",
            "--rule-filter",
            "manifests",
        ]
        direct = subprocess.run(
            [os.environ["SC_LINT_BIN"], *argv[1:]],
            capture_output=True,
            check=False,
            text=True,
        )
        wheel = subprocess.run(
            [sys.executable, "-c", "from sc_lint import run; raise SystemExit(run(__import__('sys').argv[1:]))", *argv],
            capture_output=True,
            check=False,
            text=True,
        )

        self.assertEqual(wheel.returncode, direct.returncode, wheel.stderr)
        self.assertEqual(
            json.loads(wheel.stdout)["data"],
            json.loads(direct.stdout)["data"],
        )

    def test_manifests_adapter_does_not_read_boundary_policy(self) -> None:
        adapter = MANIFEST_ADAPTER.read_text(encoding="utf-8")
        for key in (
            "manifest_dependency_allowlists",
            "global_dependency_ownership",
            "manifest_section_rules",
        ):
            with self.subTest(key=key):
                self.assertNotIn(key, adapter)


if __name__ == "__main__":
    unittest.main()
