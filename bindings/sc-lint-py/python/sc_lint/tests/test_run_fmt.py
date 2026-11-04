from __future__ import annotations

from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import sys
import unittest


from sc_lint import run_fmt
from sc_lint.run_fmt import resolve_command


class RunFmtTests(unittest.TestCase):
    def test_resolve_command_accepts_aliases(self) -> None:
        self.assertEqual(resolve_command("check"), ["just", "_fmt-check"])
        self.assertEqual(resolve_command("write"), ["just", "_fmt-write"])
        self.assertEqual(resolve_command("apply"), ["just", "_fmt-write"])

    def test_resolve_command_rejects_unknown_mode(self) -> None:
        self.assertIsNone(resolve_command("unknown"))

    def test_help_prints_usage_without_running_a_mode(self) -> None:
        stdout = StringIO()
        with redirect_stdout(stdout):
            self.assertEqual(run_fmt.main(["run_fmt", "--help"]), 0)
        self.assertIn("usage: python -m sc_lint.run_fmt [check|write|apply]", stdout.getvalue())


if __name__ == "__main__":
    unittest.main()
