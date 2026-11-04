from __future__ import annotations

from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
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

    def test_failed_command_reports_stdout_with_failure_summary(self) -> None:
        python = f'"{sys.executable}"'
        failing = f"{python} -c \"print('named cargo test failure'); raise SystemExit(1)\""
        stderr = StringIO()
        with redirect_stderr(stderr):
            self.assertEqual(run_all.main(["run_all", failing]), 1)
        self.assertIn("named cargo test failure", stderr.getvalue())

    def test_help_prints_usage_and_runs_nothing(self) -> None:
        stdout = StringIO()
        with redirect_stdout(stdout):
            self.assertEqual(run_all.main(["run_all", "--help"]), 0)
        self.assertIn("usage: python -m sc_lint.run_all COMMAND", stdout.getvalue())
