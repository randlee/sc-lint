from __future__ import annotations

from pathlib import Path
import sys
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from print_help import render_help


class PrintHelpTests(unittest.TestCase):
    def test_render_help_mentions_expected_entries(self) -> None:
        output = render_help("sc-lint")
        self.assertIn("version latest", output)
        self.assertIn("lint fast", output)
        self.assertIn("lint modules", output)
        self.assertIn("lint sc-boundary", output)
        self.assertIn("lint sc-portability", output)
        self.assertIn("lint manifests", output)
        self.assertIn("lint pytests", output)
        self.assertIn("fmt apply", output)
        self.assertNotIn("lint daemon-singleton", output)
        self.assertNotIn("lint boundaries", output)
        self.assertNotIn("view boundaries", output)


if __name__ == "__main__":
    unittest.main()
