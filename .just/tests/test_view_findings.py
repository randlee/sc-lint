from __future__ import annotations

import json
from pathlib import Path
import sys
import tempfile
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from view_findings import build_index


class ViewFindingsTests(unittest.TestCase):
    def test_build_index_collates_findings_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            findings_dir = repo_root / "artifacts/findings/line-counts"
            findings_dir.mkdir(parents=True, exist_ok=True)
            (findings_dir / "summary.json").write_text(
                json.dumps(
                    {
                        "tool": "sc-lint-line-counts",
                        "status": "pass",
                        "summary": "source file size limits satisfied",
                        "findings": [],
                    }
                ),
                encoding="utf-8",
            )

            data = build_index(repo_root)

            self.assertEqual(data["summary"], "collated 1 findings artifact set(s)")
            self.assertEqual(len(data["views"]), 1)
            self.assertEqual(data["views"][0]["tool"], "sc-lint-line-counts")
            self.assertTrue((repo_root / "artifacts/view/findings/index.json").is_file())


if __name__ == "__main__":
    unittest.main()
