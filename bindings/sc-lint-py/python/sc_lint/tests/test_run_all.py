from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest

from sc_lint import run_all


class RunAllTests(unittest.TestCase):
    def test_runs_every_command_after_a_failure_and_fails(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            marker = Path(raw) / "ran"
            python = f'"{sys.executable}"'
            failing = f"{python} -c \"raise SystemExit(3)\""
            later = f"{python} -c \"open(r'{marker}', 'w').close()\""
            self.assertEqual(run_all.main(["run_all", failing, later]), 1)
            self.assertTrue(marker.exists())

    def test_all_passing_commands_succeed(self) -> None:
        python = f'"{sys.executable}"'
        self.assertEqual(run_all.main(["run_all", f"{python} -c pass"]), 0)
