from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from view_common import relative_artifact_path
from view_common import reset_findings_dir
from view_common import reset_view_dir
from view_common import write_json
from view_common import write_text


class ViewCommonTests(unittest.TestCase):
    def test_reset_view_and_findings_dirs_recreate_target(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            findings = repo_root / "artifacts/findings/sample"
            findings.mkdir(parents=True)
            (findings / "stale.txt").write_text("stale", encoding="utf-8")

            findings_target = reset_findings_dir(repo_root, "sample")
            view_target = reset_view_dir(repo_root, "findings")

            self.assertEqual(findings_target, repo_root / "artifacts/findings/sample")
            self.assertEqual(view_target, repo_root / "artifacts/view/findings")
            self.assertFalse((findings / "stale.txt").exists())

    def test_write_text_and_json(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            text_path = repo_root / "artifacts/view/sample.txt"
            json_path = repo_root / "artifacts/findings/sample.json"

            write_text(text_path, "hello\n")
            write_json(json_path, {"name": "sc-lint"})

            self.assertEqual(text_path.read_text(encoding="utf-8"), "hello\n")
            self.assertIn('"name": "sc-lint"', json_path.read_text(encoding="utf-8"))

    def test_relative_artifact_path_is_posix(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            repo_root = Path(tempdir)
            path = repo_root / "artifacts/view/findings/index.txt"
            self.assertEqual(relative_artifact_path(repo_root, path), "artifacts/view/findings/index.txt")


if __name__ == "__main__":
    unittest.main()
