from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
REQUEST = ROOT / "tests" / "fixtures" / "configure" / "contracts" / "request.json"
BINARY = ROOT / "target" / "debug" / ("sc-lint.exe" if sys.platform == "win32" else "sc-lint")
WORKFLOW = Path(".github/workflows/sc-lint.yml")


def configure(root: Path, request: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            str(BINARY),
            "--json",
            "--root",
            str(root),
            "configure",
            "--request",
            str(request),
            *arguments,
        ],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )


class WorkflowTransformerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not BINARY.is_file():
            subprocess.run(["cargo", "build", "-q", "-p", "sc-lint"], cwd=ROOT, check=True)

    def request(self, root: Path) -> Path:
        request = json.loads(REQUEST.read_text(encoding="utf-8"))
        request["request"]["ci"]["mode"] = "generate_managed_workflow"
        path = root / "request.json"
        path.write_text(json.dumps(request), encoding="utf-8")
        return path

    def plan(self, root: Path, request: Path) -> Path:
        planned = configure(root, request)
        self.assertEqual(planned.returncode, 0, planned.stderr)
        path = root / "plan.json"
        path.write_text(planned.stdout, encoding="utf-8")
        return path

    def test_create_and_reapply_are_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            request = self.request(root)
            applied = configure(root, request, "--apply", "--plan", str(self.plan(root, request)))
            self.assertEqual(applied.returncode, 0, applied.stderr)
            workflow = (root / WORKFLOW).read_text(encoding="utf-8")
            self.assertEqual(workflow.count("uses: randlee/sc-lint@v1"), 3)
            self.assertIn("operation: setup", workflow)
            self.assertIn("operation: lint", workflow)
            self.assertIn("operation: test", workflow)
            self.assertIn("config-path: sc-lint.toml", workflow)
            self.assertNotIn("actions/checkout", workflow)

            reapplied = configure(root, request, "--apply", "--plan", str(self.plan(root, request)))
            self.assertEqual(reapplied.returncode, 0, reapplied.stderr)
            self.assertEqual((root / WORKFLOW).read_text(encoding="utf-8"), workflow)

    def test_unknown_and_near_match_workflows_are_no_write_conflicts(self) -> None:
        for contents in (
            "name: user workflow\non: [push]\n",
            "name: sc-lint\n\non:\n  pull_request:\n# near but user-owned\n",
        ):
            with self.subTest(contents=contents), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
                target = root / WORKFLOW
                target.parent.mkdir(parents=True)
                target.write_text(contents, encoding="utf-8")
                request = self.request(root)
                applied = configure(root, request, "--apply", "--plan", str(self.plan(root, request)))
                self.assertEqual(applied.returncode, 3)
                failure = json.loads(applied.stderr)
                self.assertEqual(failure["error"]["code"], "CLI.CONFIGURE_UNMANAGED_COLLISION")
                self.assertEqual(target.read_text(encoding="utf-8"), contents)
                self.assertFalse((root / "sc-lint.toml").exists())

    def test_changed_workflow_target_rejects_the_reviewed_plan_as_stale(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            request = self.request(root)
            plan = self.plan(root, request)
            target = root / WORKFLOW
            target.parent.mkdir(parents=True)
            target.write_text("name: changed after review\n", encoding="utf-8")
            applied = configure(root, request, "--apply", "--plan", str(plan))
            self.assertEqual(applied.returncode, 3)
            failure = json.loads(applied.stderr)
            self.assertEqual(failure["error"]["code"], "CLI.CONFIGURE_STALE_PLAN")
            self.assertEqual(target.read_text(encoding="utf-8"), "name: changed after review\n")
            self.assertFalse((root / "sc-lint.toml").exists())


if __name__ == "__main__":
    unittest.main()
