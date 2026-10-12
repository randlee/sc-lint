import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[5]
SKILL = ROOT / "plugins" / "sc-plan-anchor" / "skills" / "plan-anchoring"
MODULE_PATH = SKILL / "scripts" / "plan_source.py"
JUDGE_PATH = SKILL / "scripts" / "jev_judge.py"


def load_source():
    spec = importlib.util.spec_from_file_location("plan_source", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_judge():
    spec = importlib.util.spec_from_file_location("plan_source_jev_judge", JUDGE_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def sprint(identifier: str, extra: str = "") -> str:
    return f"""# Sprint {identifier} — Work

## Objective

Do the work.

## Scope boundary

Only this work.

## Authoritative deliverables

- Deliver it.

## Authoritative acceptance criteria

- It works.

## Authoritative validation

- Run tests.

## Non-closure

- Other work.

{extra}"""


def phase(rows: str) -> str:
    return f"""# Phase A — Test

## Sprint stack

| Sprint | Authoritative plan | Tracker | Relation | Closure |
| --- | --- | --- | --- | --- |
{rows}
"""


class PlanSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = load_source()
        cls.judge = load_judge()

    def fixture_repo(self, root: Path) -> Path:
        plan = root / "docs" / "plans" / "phase-a"
        plan.mkdir(parents=True)
        (plan / "sprint-a1.md").write_text(sprint("A1"), encoding="utf-8")
        (plan / "sprint-a2.md").write_text(sprint("A2", "```json\n{\"example\": true}\n```\n"), encoding="utf-8")
        (plan / "plan-phase-a.md").write_text(
            phase(
                "| A1 | [One](sprint-a1.md) | `p.1` | root | done |\n"
                "| A2 | [Two](sprint-a2.md) | `p.2` | `must_follow` A1 | done |"
            ),
            encoding="utf-8",
        )
        return plan

    def test_dogfood_file_and_directory_discover_a0_through_a7(self):
        target = "docs/plans/phase-a/plan-phase-a.md"
        by_file = self.source.discover_markdown(ROOT, target)
        by_directory = self.source.discover_markdown(ROOT, "docs/plans/phase-a")
        expected = [f"A{number}" for number in range(8)]
        self.assertEqual([item["id"] for item in by_file["sprints"]], expected)
        self.assertEqual([item["id"] for item in by_directory["sprints"]], expected)
        self.assertEqual(
            [item["normalized_sha256"] for item in by_file["sprints"]],
            [item["normalized_sha256"] for item in by_directory["sprints"]],
        )

    def test_markdown_and_tracker_copy_normalize_and_order_identically(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            plan_dir = self.fixture_repo(root)
            markdown = self.source.discover_markdown(root, "docs/plans/phase-a")
            children = []
            for item in reversed(markdown["sprints"]):
                children.append({
                    "id": "p." + item["id"][-1],
                    "description": item["text"],
                    "updated_at": "2026-10-11T00:00:00Z",
                })
            for tracker_name in ("bd", "br"):
                tracker = self.source.discover_tracker(children, phase="phase-a", tracker=tracker_name, epic="p")
                self.assertEqual([item["id"] for item in tracker["sprints"]], ["A1", "A2"])
                parity = self.source.compare_parity(markdown, tracker)
                self.assertTrue(parity["equal"])
                self.assertIn('```json\n{"example": true}\n```', tracker["sprints"][1]["text"])

    def test_tracker_epic_discovery_uses_fixed_argv_and_direct_children(self):
        epic = [{
            "id": "p",
            "issue_type": "epic",
            "description": "# Phase A — Test\n",
            "updated_at": "rev-epic",
            "dependents": [
                {"id": "p.2", "dependency_type": "parent-child"},
                {"id": "unrelated", "dependency_type": "blocks"},
                {"id": "p.1", "dependency_type": "parent-child"},
            ],
        }]
        children = [
            {
                "id": "p.2",
                "description": sprint("A2"),
                "updated_at": "rev-2",
                "dependencies": [
                    {"id": "p.1", "dependency_type": "blocks"}
                ],
            },
            {"id": "p.1", "description": sprint("A1"), "updated_at": "rev-1"},
        ]
        with mock.patch.object(
            self.source, "_tracker_json", side_effect=[epic, children]
        ) as run:
            plan = self.source.discover_tracker_epic(ROOT, "br", "p")
        self.assertEqual([item["id"] for item in plan["sprints"]], ["A1", "A2"])
        self.assertEqual(
            [item["dependency_relation"] for item in plan["sprints"]],
            ["root", "must_follow A1"],
        )
        self.assertEqual(plan["source_revision"], "rev-epic")
        self.assertEqual(run.call_args_list[0].args[2], ["show", "p"])
        self.assertEqual(run.call_args_list[1].args[2], ["show", "p.2", "p.1"])

    def test_copy_verifies_or_updates_with_revision_guard(self):
        markdown_item = {
            "id": "A1",
            "text": sprint("A1"),
            "path": "docs/sprint-a1.md",
            "normalized_sha256": self.source.digest_text(sprint("A1")),
        }
        markdown = {"sprints": [markdown_item]}
        tracker_item = {
            "id": "A1",
            "text": sprint("A1"),
            "normalized_sha256": markdown_item["normalized_sha256"],
            "recorded_source_sha256": None,
            "tracker_id": "p.1",
            "updated_at": "rev-1",
            "external_ref": "plan-copy:docs/sprint-a1.md",
        }
        tracker = {"source_kind": "br", "sprints": [tracker_item]}
        dry = self.source.copy_plan_to_tracker(ROOT, markdown, tracker)
        self.assertEqual(dry["actions"][0]["status"], "update_required")
        with mock.patch.object(self.source, "_tracker_json", return_value=[{}]) as run:
            applied = self.source.copy_plan_to_tracker(ROOT, markdown, tracker, apply=True)
        self.assertEqual(applied["actions"][0]["status"], "updated")
        argv = run.call_args.args[2]
        self.assertEqual(argv[0:2], ["update", "p.1"])
        self.assertIn("--description-file", argv)
        self.assertEqual(argv[-2:], ["--if-unchanged", "rev-1"])

        tracker_item["external_ref"] = (
            "plan-copy:docs/sprint-a1.md@sha256:"
            + markdown_item["normalized_sha256"]
        )
        verified = self.source.copy_plan_to_tracker(ROOT, markdown, tracker)
        self.assertEqual(verified["actions"][0]["status"], "verified")

    def test_canonical_jev_body_excludes_source_metadata_for_all_sources(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            self.fixture_repo(root)
            markdown = self.source.discover_markdown(root, "docs/plans/phase-a")
            base = markdown["sprints"][0]
            anchor = {"id": "ANC-001", "statement": "The runner must be shared.", "scope": "runner"}
            questions = {"q": {"instructions": "Is it shared?", "options": {"yes": "yes", "no": "no"}}}
            children = [{"id": "p.1", "description": base["text"]}]
            sources = [base]
            for kind in ("bd", "br"):
                sources.append(
                    self.source.discover_tracker(
                        children, phase="phase-a", tracker=kind, epic="p"
                    )["sprints"][0]
                )
            bodies = [
                self.source.canonical_jev_body(
                    "jev-1.13.0", anchor, item, questions
                )
                for item in sources
            ]
            self.assertEqual(bodies[0], bodies[1])
            self.assertEqual(bodies[1], bodies[2])
            self.assertNotIn(b"source_kind", bodies[0])
            self.assertNotIn(b"tracker_id", bodies[0])
            decoded = json.loads(bodies[0])
            self.assertEqual(decoded["state"]["item"]["id"], "A1")
            self.assertEqual(set(decoded["state"]), {"anchor", "item"})
            expected = self.judge.encode_body({
                "model": "jev-1.13.0",
                "state": self.judge.build_state(anchor, base),
                "questions": questions,
            })
            self.assertEqual(bodies[0], expected)

    def test_invalid_phase_structures_fail_before_delegation(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            plan_dir = self.fixture_repo(root)
            index = plan_dir / "plan-phase-a.md"
            original = index.read_text(encoding="utf-8")

            for replacement, code in (
                (original.replace("| --- | --- | --- | --- | --- |", "| bad |"), "SOURCE.PHASE_TABLE"),
                (original.replace("sprint-a2.md", "sprint-a1.md"), "SOURCE.DUPLICATE_PATH"),
                (original.replace("sprint-a2.md", "missing.md"), "SOURCE.PATH"),
                (original.replace("sprint-a2.md", "/tmp/outside.md"), "SOURCE.PATH"),
                (original + original[original.index("| Sprint |"):], "SOURCE.PHASE_TABLE"),
            ):
                index.write_text(replacement, encoding="utf-8")
                with self.assertRaises(self.source.SourceError) as caught:
                    self.source.discover_markdown(root, "docs/plans/phase-a")
                self.assertEqual(caught.exception.code, code)
                index.write_text(original, encoding="utf-8")

            (plan_dir / "plan-phase-extra.md").write_text(original, encoding="utf-8")
            with self.assertRaises(self.source.SourceError) as caught:
                self.source.discover_markdown(root, "docs/plans/phase-a")
            self.assertEqual(caught.exception.code, "SOURCE.PHASE_INDEX")

    def test_broken_sections_duplicate_ids_and_symlink_escape_fail(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            plan_dir = self.fixture_repo(root)
            target = plan_dir / "sprint-a1.md"
            target.write_text(target.read_text(encoding="utf-8").replace("## Non-closure", "## Notes"), encoding="utf-8")
            with self.assertRaises(self.source.SourceError) as caught:
                self.source.discover_markdown(root, "docs/plans/phase-a")
            self.assertEqual(caught.exception.code, "SOURCE.SECTION")

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            plan_dir = self.fixture_repo(root)
            outside = Path(td).parent / "outside-plan-anchor-test.md"
            outside.write_text(sprint("A1"), encoding="utf-8")
            target = plan_dir / "sprint-a1.md"
            target.unlink()
            target.symlink_to(outside)
            try:
                with self.assertRaises(self.source.SourceError) as caught:
                    self.source.discover_markdown(root, "docs/plans/phase-a")
                self.assertEqual(caught.exception.code, "SOURCE.PATH")
            finally:
                outside.unlink(missing_ok=True)

        duplicate = [{"id": "x1", "description": sprint("A1")}, {"id": "x2", "description": sprint("A1")}]
        with self.assertRaises(self.source.SourceError) as caught:
            self.source.discover_tracker(duplicate, phase="phase-a", tracker="br", epic="x")
        self.assertEqual(caught.exception.code, "SOURCE.DUPLICATE_ID")

    def test_drift_copy_conflict_digest_and_size_are_fail_closed(self):
        item = {
            "id": "A1",
            "text": sprint("A1"),
            "path": "docs/sprint-a1.md",
            "normalized_sha256": self.source.digest_text(sprint("A1")),
        }
        update = self.source.safe_copy_update(item, None)
        self.assertEqual(update["description"], item["text"])
        changed = {"tracker_id": "p.1", "normalized_sha256": "b" * 64, "recorded_source_sha256": "a" * 64}
        with self.assertRaises(self.source.SourceError) as caught:
            self.source.safe_copy_update(item, changed)
        self.assertEqual(caught.exception.code, "SOURCE.COPY_CONFLICT")

        left = {"sprints": [{"id": "A1", "normalized_sha256": "a" * 64}]}
        right = {"sprints": [{"id": "A1", "normalized_sha256": "b" * 64}]}
        with self.assertRaises(self.source.SourceError) as caught:
            self.source.compare_parity(left, right)
        self.assertEqual(caught.exception.code, "SOURCE.PARITY_DRIFT")

        oversized = sprint("A1") + ("x" * self.source.MAX_PLAN_BYTES)
        with self.assertRaises(self.source.SourceError) as caught:
            self.source.shape(oversized)
        self.assertEqual(caught.exception.code, "PLAN.TOO_LARGE")
        self.assertTrue(caught.exception.data["over_limit"])
        self.assertFalse(caught.exception.data["truncated"])


if __name__ == "__main__":
    unittest.main()
