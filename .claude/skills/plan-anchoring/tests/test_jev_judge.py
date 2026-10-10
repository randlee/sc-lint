"""Unit tests for scripts/jev_judge.py. Standard library only; no network is ever touched.

The Jev transport (`jev_judge.call_jev`) is replaced with a stub, so these tests prove the deterministic
half of the skill (IDs, gating, policy, coverage, diffs, reports), not Jev's own behaviour.
"""
import contextlib
import datetime as _dt
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import jev_judge as jj  # noqa: E402

PLAN_ROLES = {"relevance": "relevant", "compliance": "addressed", "conflict": "contradicted"}
DEV_ROLES = {"relevance": "relevant", "compliance": "implemented", "conflict": "violated", "evidence": "evidenced"}
OPTIONS = ("yes", "no", "unclear")
STATEMENT = "copy the libraries from ATM as the starting point and remove the ATM-specific code"
LIVE_ENV = {"TYPESAFE_API_KEY": "sk-test-0001", "PLAN_ANCHOR_ALLOW_REMOTE_JUDGE": "1"}


def answer(choice, p=0.9, options=OPTIONS):
    rest = round((1 - p) / (len(options) - 1), 4)
    probs = {o: (p if o == choice else rest) for o in options}
    return {"type": "choice", "choice": choice, "confidence": p, "probabilities": probs}


def plan_answers(rel="yes", comp="yes", conf="no", p=0.9, rel_p=None, comp_p=None, conf_p=None):
    return {"relevant": answer(rel, rel_p if rel_p is not None else p),
            "addressed": answer(comp, comp_p if comp_p is not None else p),
            "contradicted": answer(conf, conf_p if conf_p is not None else p)}


def dev_answers(rel="yes", comp="yes", conf="no", evid="yes", p=0.9, evid_p=None):
    return {"relevant": answer(rel, p), "implemented": answer(comp, p), "violated": answer(conf, p),
            "evidenced": answer(evid, evid_p if evid_p is not None else p)}


def anchor(aid="ANC-001", status="locked", source_type="ANC", source_ref=None, statement=STATEMENT, **extra):
    rec = {
        "id": aid, "source_type": source_type, "source_ref": source_ref, "statement": statement,
        "stated_by": "Rand", "stated_at": "2026-10-09", "context": "planning session",
        "summary": "start from ATM", "scope": "the runtime crates", "deterministic_check": "",
        "fixtures": {"pass": [], "fail": []}, "question_overrides": {}, "version": 1, "status": status,
        "promoted_to": None, "beads": {"anchor": None, "plan_questions": None, "dev_questions": None}, "history": [],
    }
    rec.update(extra)
    return rec


def record(*anchors):
    return {"schema": "plan-anchoring/anchors/1", "plan": "phase-current", "intent_ref": "doc/plan/phase-current/intent.md",
            "anchors": list(anchors)}


def write(d, name, value):
    path = Path(d) / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value) if not isinstance(value, str) else value, encoding="utf-8")
    return str(path)


def run_cli(argv):
    """Run main() in-process; returns (exit code, parsed JSON or raw text)."""
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        code = jj.main(argv)
    out = buf.getvalue()
    try:
        return code, json.loads(out)
    except json.JSONDecodeError:
        return code, out


def env(**values):
    return mock.patch.dict(os.environ, values, clear=True)


class Transport:
    """Stub for jev_judge.call_jev: a queue of responses (dicts) or JudgeErrors, recorded per call."""

    def __init__(self, *script):
        self.script = list(script)
        self.bodies = []

    def __call__(self, body, api_key, timeout):
        self.bodies.append(json.loads(json.dumps(body)))
        step = self.script.pop(0) if self.script else self.default()
        if isinstance(step, Exception):
            raise step
        return step

    def default(self):
        return {"model": "jev-1.13.0", "answers": plan_answers()}


def http(status, text="", retry_after=None):
    return jj.http_error(status, text, retry_after)


class Judged:
    """Run `judge` against a stubbed transport with the opt-in satisfied; exposes the run and call log."""

    def __init__(self, d, transport, anchors=None, extra_args=(), item_text="Sprint 2 copies the ATM runtime crates.", env_values=None):
        self.anchors_path = write(d, "anchors.json", record(*(anchors or [anchor()])))
        self.item_path = write(d, "item.txt", item_text)
        self.sleeps = []
        argv = ["judge", "--anchors", self.anchors_path, "--gate", "plan", "--all", "--item-id", "bd-42",
                "--item-file", self.item_path, "--run-dir", str(Path(d) / "runs"), *extra_args]
        with env(**(LIVE_ENV if env_values is None else env_values)), mock.patch.object(jj, "call_jev", transport), \
                mock.patch.object(jj, "_sleep", self.sleeps.append):
            self.code, self.out = run_cli(argv)
        self.run = self.out.get("data") if isinstance(self.out, dict) else None
        self.result = self.run["results"][0] if self.run and self.run.get("results") else None


# --------------------------------------------------------------------------- #
class AddTests(unittest.TestCase):
    def test_next_anc_id_allocation(self):
        self.assertEqual(jj.next_anc_id(record()), "ANC-001")
        rec = record(anchor("ANC-003"), anchor("ANC-010"), anchor("REQ-014", source_type="REQ", source_ref="REQ-014"), {"id": 7})
        self.assertEqual(jj.next_anc_id(rec), "ANC-011")
        self.assertEqual(jj.next_anc_id(record(anchor("ANC-1000"))), "ANC-1001")

    def test_add_creates_file_and_allocates_ids(self):
        with tempfile.TemporaryDirectory() as d:
            path = str(Path(d) / "anchors.json")
            code, out = run_cli(["add", "--anchors", path, "--statement", "  no new dependency for this  ", "--status", "locked"])
            self.assertEqual(code, 0)
            self.assertEqual(out["data"]["anchor"]["id"], "ANC-001")
            self.assertEqual(out["data"]["anchor"]["statement"], "no new dependency for this")
            self.assertEqual(out["data"]["anchor"]["stated_at"], _dt.date.today().isoformat())
            code, out = run_cli(["add", "--anchors", path, "--statement", "keep the CLI thin"])
            self.assertEqual(out["data"]["anchor"]["id"], "ANC-002")
            self.assertFalse(Path(path + ".tmp").exists())
            self.assertEqual(len(json.loads(Path(path).read_text())["anchors"]), 2)

    def test_register_anchors_keep_their_id_and_refuse_duplicates(self):
        with tempfile.TemporaryDirectory() as d:
            path = str(Path(d) / "anchors.json")
            code, out = run_cli(["add", "--anchors", path, "--statement", "x", "--source-type", "REQ"])
            self.assertEqual((code, out["error"]["code"]), (2, "VALIDATION.INPUT"))
            code, out = run_cli(["add", "--anchors", path, "--statement", "x", "--source-type", "REQ", "--source-ref", "REQ-014"])
            self.assertEqual(out["data"]["anchor"]["id"], "REQ-014")
            code, out = run_cli(["add", "--anchors", path, "--statement", "x", "--source-type", "REQ", "--source-ref", "REQ-014"])
            self.assertEqual((code, out["error"]["code"]), (2, "ANCHOR.DUPLICATE"))

    def test_add_refuses_empty_statement_and_retired_status(self):
        with tempfile.TemporaryDirectory() as d:
            path = str(Path(d) / "anchors.json")
            code, out = run_cli(["add", "--anchors", path, "--statement", "   "])
            self.assertEqual(out["error"]["code"], "VALIDATION.INPUT")
            code, out = run_cli(["add", "--anchors", path, "--statement", "x", "--status", "retired"])
            self.assertEqual((code, out["error"]["code"]), (2, "VALIDATION.INPUT"))


class ValidateTests(unittest.TestCase):
    def test_problems_are_listed(self):
        rec = record(
            anchor("ANC-1", status="locked"),
            anchor("ANC-002", status="bogus", statement="  "),
            anchor("ANC-002"),
            anchor("REQ-014", source_type="REQ", source_ref=None),
            anchor("ANC-003", status="promoted"),
            anchor("ANC-004", source_type="XYZ", stated_at="", question_overrides={"prod": {}}),
            "not an object",
        )
        rec["schema"] = "wrong"
        problems = jj.validate_record(rec)
        joined = "\n".join(problems)
        for needle in ("schema must be", "ANC-1: ANC ids look like", "ANC-002: status must be", "ANC-002: statement (verbatim) is empty",
                       "ANC-002: duplicate id", "REQ-014: REQ/ADR anchors need source_ref", "ANC-003: promoted anchors need promoted_to",
                       "ANC-004: source_type must be", "ANC-004: stated_at missing", "question_overrides key must be plan or dev",
                       "anchors[6]: must be an object"):
            self.assertIn(needle, joined)
        self.assertEqual(jj.validate_record(record(anchor())), [])
        self.assertEqual(jj.validate_record({"schema": "plan-anchoring/anchors/1", "anchors": {}}), ["'anchors' must be a list"])

    def test_validate_cli_exit_codes_and_warnings(self):
        with tempfile.TemporaryDirectory() as d:
            path = write(d, "anchors.json", record(anchor(status="proposed")))
            code, out = run_cli(["validate", "--anchors", path])
            self.assertEqual(code, 0)
            self.assertIn("no locked anchors", out["data"]["warnings"])
            self.assertIn("intent_ref file not found: doc/plan/phase-current/intent.md", out["data"]["warnings"])
            Path(d, "intent.md").write_text("# intent\n")
            code, out = run_cli(["validate", "--anchors", path])
            self.assertEqual([w for w in out["data"]["warnings"] if "intent_ref" in w], [])
            path = write(d, "bad.json", record(anchor(status="nope")))
            code, out = run_cli(["validate", "--anchors", path])
            self.assertEqual((code, out["success"], out["error"]["code"]), (1, False, "VALIDATION.ANCHORS"))
            self.assertTrue(out["data"]["problems"])


class DecideTests(unittest.TestCase):
    def decide(self, answers, roles=PLAN_ROLES, t=0.6):
        return jj.decide(answers, roles, t)

    def test_row1_not_applicable(self):
        self.assertEqual(self.decide(plan_answers(rel="no", comp="no", conf="no"))[0], "not_applicable")
        # relevance "no" below threshold falls through to underspecified
        self.assertEqual(self.decide(plan_answers(rel="no", rel_p=0.5))[0], "underspecified")

    def test_row2_contradicted_outranks_underspecification(self):
        self.assertEqual(self.decide(plan_answers(rel="yes", comp="no", conf="yes"))[0], "contradicted")
        self.assertEqual(self.decide(plan_answers(rel="unclear", comp="unclear", conf="yes"))[0], "contradicted")
        self.assertEqual(self.decide(plan_answers(rel="yes", comp="no", conf="yes", conf_p=0.55))[0], "not_addressed")

    def test_row3_underspecified(self):
        self.assertEqual(self.decide(plan_answers(rel="unclear"))[0], "underspecified")
        self.assertEqual(self.decide(plan_answers(rel_p=0.59))[0], "underspecified")
        self.assertEqual(self.decide(plan_answers(comp="unclear"))[0], "underspecified")
        verdict, min_conf = self.decide(plan_answers(comp_p=0.3))
        self.assertEqual((verdict, min_conf), ("underspecified", 0.3))

    def test_row4_not_addressed(self):
        self.assertEqual(self.decide(plan_answers(comp="no"))[0], "not_addressed")

    def test_row5_asserted_only_needs_evidence_role(self):
        self.assertEqual(self.decide(dev_answers(evid="no"), DEV_ROLES)[0], "asserted_only")
        self.assertEqual(self.decide(dev_answers(evid="no", evid_p=0.4), DEV_ROLES)[0], "satisfied")
        self.assertEqual(self.decide(dev_answers(evid="unclear"), DEV_ROLES)[0], "satisfied")
        no_evidence_role = {k: v for k, v in DEV_ROLES.items() if k != "evidence"}
        self.assertEqual(self.decide(dev_answers(evid="no"), no_evidence_role)[0], "satisfied")

    def test_row6_satisfied_and_min_confidence(self):
        verdict, min_conf = self.decide(plan_answers(rel_p=0.95, comp_p=0.7, conf_p=0.8))
        self.assertEqual((verdict, min_conf), ("satisfied", 0.7))

    def test_row7_fallthrough(self):
        answers = plan_answers()
        answers["addressed"]["choice"] = "maybe"
        self.assertEqual(self.decide(answers)[0], "underspecified")
        self.assertEqual(self.decide({}), ("underspecified", 0.0))

    def test_threshold_boundaries(self):
        self.assertEqual(self.decide(plan_answers(p=0.6), t=0.6)[0], "satisfied")
        self.assertEqual(self.decide(plan_answers(p=0.6), t=0.61)[0], "underspecified")
        self.assertEqual(self.decide(plan_answers(rel="no", p=0.1), t=0.0)[0], "not_applicable")

    def test_noul_fallback(self):
        self.assertEqual(jj.answer_choice({"type": "noul", "noul": 0.8}), ("yes", 0.6000000000000001))
        self.assertEqual(jj.answer_choice({"type": "noul", "probability": 0.2}), ("no", 0.6))
        self.assertEqual(jj.answer_choice({"noul": 0.5}), ("yes", 0.0))
        answers = plan_answers()
        answers["contradicted"] = {"type": "noul", "noul": 0.9}
        self.assertEqual(self.decide(answers)[0], "contradicted")
        self.assertIsNone(jj.answer_problem({"type": "noul", "noul": 0.3}, {"criteria": {}}))
        self.assertIsNotNone(jj.answer_problem({"type": "noul"}, {"criteria": {}}))


class ResponseValidationTests(unittest.TestCase):
    question = {"type": "choice", "criteria": {"yes": "", "no": "", "unclear": ""}}

    def test_good_answer_passes_and_rounding_is_tolerated(self):
        self.assertIsNone(jj.answer_problem(answer("yes"), self.question))
        ok = answer("yes")
        ok["probabilities"] = {"yes": 0.94, "no": 0.05, "unclear": 0.0}
        self.assertIsNone(jj.answer_problem(ok, self.question))

    def test_problems_named(self):
        bad = answer("yes")
        bad["probabilities"]["yes"] = 0.5
        self.assertRegex(jj.answer_problem(bad, self.question), "sum to")
        self.assertRegex(jj.answer_problem(answer("maybe"), self.question), "not one of")
        self.assertRegex(jj.answer_problem({"type": "choice", "choice": "yes", "confidence": "0.9", "probabilities": {}}, self.question), "confidence")
        self.assertRegex(jj.answer_problem({"type": "choice", "choice": "yes", "confidence": True, "probabilities": {}}, self.question), "confidence")
        extra = answer("yes")
        extra["probabilities"]["other"] = 0.0
        self.assertRegex(jj.answer_problem(extra, self.question), "keys differ")
        self.assertRegex(jj.answer_problem({"type": "score", "choice": "yes"}, self.question), "not choice or noul")
        self.assertRegex(jj.answer_problem("yes", self.question), "not an object")

    def test_validate_response_requires_exact_answer_ids(self):
        questions = {"relevant": self.question, "addressed": self.question, "contradicted": self.question}
        with self.assertRaises(jj.JudgeError) as cm:
            jj.validate_response({"answers": {"relevant": answer("yes")}}, questions)
        self.assertEqual(cm.exception.code, "JEV.RESPONSE_INVALID")
        self.assertTrue(cm.exception.recoverable)
        extra = dict(plan_answers(), bonus=answer("yes"))
        with self.assertRaises(jj.JudgeError):
            jj.validate_response({"answers": extra}, questions)
        with self.assertRaises(jj.JudgeError):
            jj.validate_response({"answers": []}, questions)
        with self.assertRaises(jj.JudgeError):
            jj.validate_response(["not", "a", "dict"], questions)
        self.assertIs(jj.validate_response({"model": "x", "answers": plan_answers()}, questions)["answers"]["relevant"]["choice"], "yes")

    def test_shipped_fixture_passes_validation(self):
        fixture = json.loads((Path(__file__).resolve().parents[1] / "assets" / "examples" / "answers.fixture.json").read_text())
        plan = jj.load_question_set("plan")
        for aid, answers in fixture.items():
            jj.validate_response({"answers": answers}, plan["questions"])


class HttpErrorTests(unittest.TestCase):
    def test_bodies_withheld_except_422(self):
        e401 = http(401, "secret body")
        self.assertEqual(e401.code, "AUTH.CREDENTIALS")
        self.assertNotIn("secret", str(e401))
        e422 = http(422, "x" * 400)
        self.assertEqual(e422.code, "JEV.VALIDATION")
        self.assertIn("x" * 300, str(e422))
        self.assertNotIn("x" * 301, str(e422))
        e500 = http(500, "stack trace")
        self.assertEqual((e500.code, e500.recoverable, str(e500)), ("JEV.HTTP", True, "Jev HTTP 500; response body withheld"))
        self.assertEqual((http(429).code, http(529).code, http(418).recoverable), ("JEV.RATE_LIMITED", "JEV.OVERLOADED", False))
        self.assertTrue(jj.is_transient(e500))
        self.assertFalse(jj.is_transient(http(418)))

    def test_context_overflow_detected_by_body(self):
        for body in ('{"error": "max_tokens_exceeded"}', "Token budget exceeded for state", "The TOKEN count EXCEEDS the limit"):
            self.assertEqual(http(422, body).code, "JEV.CONTEXT_EXCEEDED", body)
            self.assertEqual(http(400, body).code, "JEV.CONTEXT_EXCEEDED", body)
        self.assertEqual(http(500, "max_tokens_exceeded").code, "JEV.HTTP")
        self.assertEqual(http(422, "field 'questions.addressed.criteria' is invalid").code, "JEV.VALIDATION")

    def test_retry_after_parsing(self):
        self.assertEqual(jj.parse_retry_after("3"), 3.0)
        self.assertEqual(jj.parse_retry_after("900"), jj.MAX_RETRY_AFTER)
        self.assertIsNone(jj.parse_retry_after("Wed, 21 Oct 2026"))
        self.assertIsNone(jj.parse_retry_after(None))
        self.assertIsNone(jj.parse_retry_after("-1"))


class OverrideTests(unittest.TestCase):
    def test_merge_replaces_by_id_and_roles(self):
        base = jj.load_question_set("plan")
        merged = jj.merge_overrides(base, {"questions": {"addressed": {"type": "choice", "instructions": "override", "criteria": {}}}})
        self.assertEqual(merged["questions"]["addressed"]["instructions"], "override")
        self.assertEqual(merged["questions"]["relevant"], base["questions"]["relevant"])
        self.assertEqual(merged["roles"], base["roles"])
        self.assertIs(jj.merge_overrides(base, {})["questions"] is base["questions"], False)
        renamed = jj.merge_overrides(base, {"questions": {"covers": base["questions"]["addressed"]}, "roles": {"compliance": "covers"}})
        self.assertEqual(renamed["roles"]["compliance"], "covers")
        jj.check_roles(renamed, "t")

    def test_malformed_overrides_are_refused(self):
        base = jj.load_question_set("plan")
        for bad in (["list"], {"questions": ["list"]}, {"roles": "compliance=x"}):
            with self.assertRaises(jj.JudgeError) as cm:
                jj.merge_overrides(base, bad)
            self.assertEqual(cm.exception.code, "INPUT.MALFORMED")

    def test_roles_rechecked_after_merge(self):
        base = jj.load_question_set("plan")
        bad = jj.merge_overrides(base, {"roles": {"compliance": "covers"}})
        with self.assertRaises(jj.JudgeError) as cm:
            jj.check_roles(bad, "t")
        self.assertEqual(cm.exception.code, "QUESTIONS.ROLES")
        dev = jj.load_question_set("dev")
        with self.assertRaises(jj.JudgeError) as cm:
            jj.check_roles(jj.merge_overrides(dev, {"roles": {"evidence": "missing"}}), "t")
        self.assertEqual(cm.exception.code, "QUESTIONS.ROLES")
        with self.assertRaises(jj.JudgeError) as cm:
            jj.check_roles({"roles": PLAN_ROLES, "questions": {}}, "t")
        self.assertEqual(cm.exception.code, "INPUT.MALFORMED")

    def test_precedence_record_then_overrides_file(self):
        base = jj.load_question_set("plan")
        with tempfile.TemporaryDirectory() as d:
            a = anchor(question_overrides={"plan": {"questions": {"addressed": {"type": "choice", "instructions": "from record", "criteria": {"yes": "", "no": "", "unclear": ""}}}}})
            anchors_path = write(d, "anchors.json", record(a))
            fixture_path = write(d, "answers.json", {"*": plan_answers()})
            item_path = write(d, "item.txt", "Sprint 2 copies the ATM crates.")
            common = ["judge", "--anchors", anchors_path, "--gate", "plan", "--all", "--item-id", "bd-42", "--item-file", item_path,
                      "--answers-file", fixture_path, "--dry-run"]
            code, out = run_cli(common + ["--out", str(Path(d) / "r1.json")])
            self.assertEqual(out["data"]["requests"][0]["questions"]["addressed"]["instructions"], "from record")
            entry = out["data"]["results"][0]
            self.assertTrue(entry["question_set"]["overridden"])
            self.assertEqual(entry["questions"]["addressed"]["instructions"], "from record")
            override_path = write(d, "overrides.json", {"ANC-001": {"questions": {"addressed": {"type": "choice", "instructions": "from bead", "criteria": {"yes": "", "no": "", "unclear": ""}}}}})
            code, out2 = run_cli(common + ["--overrides-file", override_path, "--out", str(Path(d) / "r2.json")])
            self.assertEqual(out2["data"]["requests"][0]["questions"]["addressed"]["instructions"], "from bead")
            self.assertNotEqual(entry["questions_sha256"], out2["data"]["results"][0]["questions_sha256"])
            self.assertEqual(entry["questions_sha256"], jj.questions_digest(jj.merge_overrides(base, a["question_overrides"]["plan"])["questions"]))
            # a bead override that renames a question without updating roles aborts before anything is sent
            bad_path = write(d, "bad.json", {"ANC-001": {"roles": {"compliance": "nope"}}})
            code, out3 = run_cli(common + ["--overrides-file", bad_path])
            self.assertEqual((code, out3["error"]["code"]), (2, "QUESTIONS.ROLES"))

    def test_unoverridden_entries_carry_only_the_digest(self):
        with tempfile.TemporaryDirectory() as d:
            j = Judged(d, Transport())
            self.assertFalse(j.result["question_set"]["overridden"])
            self.assertNotIn("questions", j.result)
            self.assertEqual(len(j.result["questions_sha256"]), 64)


class JudgeFixtureTests(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.TemporaryDirectory()
        self.anchors = write(self.d.name, "anchors.json", record(anchor(), anchor("REQ-014", source_type="REQ", source_ref="REQ-014", statement="All public CLI commands must support --json output."), anchor("ANC-002", status="proposed")))
        self.item = write(self.d.name, "item.txt", "Sprint 2 copies the ATM runtime crates into crates/runtime.")
        self.run_dir = str(Path(self.d.name) / "runs")

    def tearDown(self):
        self.d.cleanup()

    def judge(self, *extra):
        with env():
            return run_cli(["judge", "--anchors", self.anchors, "--gate", "plan", "--item-id", "bd-42", "--item-file", self.item,
                            "--run-dir", self.run_dir, *extra])

    def test_fixture_answers_by_id_and_star(self):
        fixture = write(self.d.name, "answers.json", {"ANC-001": plan_answers(comp="no"), "*": plan_answers(rel="no")})
        code, out = self.judge("--all", "--answers-file", fixture)
        run = out["data"]
        self.assertEqual(code, 1)
        verdicts = {r["anchor_id"]: r["verdict"] for r in run["results"]}
        self.assertEqual(verdicts, {"ANC-001": "not_addressed", "REQ-014": "not_applicable"})
        self.assertEqual(run["results"][0]["anchor_id"], "ANC-001")  # failing first
        self.assertEqual(run["summary"], {"counts": {"not_addressed": 1, "not_applicable": 1}, "failing": ["ANC-001"], "errors": [], "shortened": [], "pass": False})
        self.assertTrue(run["fixture"])
        self.assertEqual(run["model_answered"], "fixture")
        self.assertEqual(run["results"][0]["attempts"], 1)
        self.assertEqual(run["path"], run["run_path"])
        self.assertTrue(Path(run["path"]).is_file())
        self.assertTrue(Path(run["path"]).name.startswith("plan-") and Path(run["path"]).name.endswith("-bd-42.json"))

    def test_passing_run_exits_zero(self):
        fixture = write(self.d.name, "answers.json", {"*": plan_answers()})
        code, out = self.judge("--anchor", "ANC-001", "--answers-file", fixture)
        self.assertEqual((code, out["data"]["summary"]["pass"]), (0, True))

    def test_missing_fixture_entry_is_an_error_verdict(self):
        fixture = write(self.d.name, "answers.json", {"REQ-014": plan_answers()})
        code, out = self.judge("--all", "--answers-file", fixture)
        by_id = {r["anchor_id"]: r for r in out["data"]["results"]}
        self.assertEqual(by_id["ANC-001"]["verdict"], "error")
        self.assertEqual(by_id["ANC-001"]["error"]["code"], "FIXTURE.MISSING")
        self.assertEqual(by_id["REQ-014"]["verdict"], "satisfied")
        self.assertEqual((code, out["data"]["summary"]["errors"]), (1, ["ANC-001"]))

    def test_malformed_fixture_answer_is_response_invalid(self):
        bad = plan_answers()
        bad["relevant"]["probabilities"]["yes"] = 0.1
        fixture = write(self.d.name, "answers.json", {"*": bad})
        code, out = self.judge("--anchor", "ANC-001", "--answers-file", fixture)
        self.assertEqual(out["data"]["results"][0]["error"]["code"], "JEV.RESPONSE_INVALID")

    def test_explicit_anchor_must_be_locked_unless_included(self):
        fixture = write(self.d.name, "answers.json", {"*": plan_answers()})
        code, out = self.judge("--anchor", "ANC-002", "--answers-file", fixture)
        self.assertEqual((code, out["error"]["code"]), (2, "ANCHOR.NOT_LOCKED"))
        code, out = self.judge("--anchor", "ANC-002", "--anchor", "ANC-002", "--include-unlocked", "--answers-file", fixture)
        self.assertEqual([r["anchor_id"] for r in out["data"]["results"]], ["ANC-002"])  # de-duplicated
        code, out = self.judge("--anchor", "ANC-999", "--answers-file", fixture)
        self.assertEqual(out["error"]["code"], "ANCHOR.UNKNOWN")

    def test_all_skips_proposed_and_requires_some_anchor(self):
        fixture = write(self.d.name, "answers.json", {"*": plan_answers()})
        code, out = self.judge("--all", "--answers-file", fixture)
        self.assertEqual(sorted(r["anchor_id"] for r in out["data"]["results"]), ["ANC-001", "REQ-014"])
        code, out = self.judge("--answers-file", fixture)
        self.assertEqual(out["error"]["code"], "VALIDATION.INPUT")
        only_proposed = write(self.d.name, "p.json", record(anchor(status="proposed")))
        with env():
            code, out = run_cli(["judge", "--anchors", only_proposed, "--gate", "plan", "--all", "--item-id", "x", "--item-file", self.item, "--answers-file", fixture])
        self.assertEqual(out["error"]["code"], "ANCHOR.NONE")

    def test_dry_run_writes_requests_and_totals_and_exits_zero(self):
        code, out = self.judge("--all", "--dry-run")
        run = out["data"]
        self.assertEqual(code, 0)
        self.assertTrue(run["dry_run"])
        self.assertFalse(run["summary"]["pass"])
        self.assertEqual(len(run["requests"]), 2)
        self.assertEqual(run["requests"][0]["state"]["item"]["id"], "bd-42")
        self.assertEqual(set(run["requests"][0]["questions"]), {"relevant", "addressed", "contradicted"})
        self.assertEqual(run["totals"]["requests"], 2)
        self.assertEqual(run["totals"]["bytes"], sum(len(jj.encode_body(b)) for b in run["requests"]))
        self.assertEqual(run["totals"]["max_request_bytes"], 24000)
        self.assertTrue(Path(run["run_path"]).is_file() and Path(run["path"]).parent == Path(self.run_dir))
        self.assertIsNone(run["results"][0]["verdict"])
        self.assertEqual(run["results"][0]["attempts"], 0)
        with self.assertRaises(jj.JudgeError):
            jj.cmd_coverage(jj.argparse.Namespace(runs=[self.run_dir], gate="", format="json"))  # dry runs never count

    def test_judge_refuses_invalid_record_and_options(self):
        bad = write(self.d.name, "bad.json", record(anchor(status="nope")))
        fixture = write(self.d.name, "answers.json", {"*": plan_answers()})
        with env():
            code, out = run_cli(["judge", "--anchors", bad, "--gate", "plan", "--all", "--item-id", "x", "--item-file", self.item, "--answers-file", fixture])
        self.assertEqual((code, out["error"]["code"]), (2, "VALIDATION.ANCHORS"))
        code, out = self.judge("--all", "--answers-file", fixture, "--threshold", "1.5")
        self.assertEqual(out["error"]["code"], "VALIDATION.INPUT")
        code, out = self.judge("--all", "--answers-file", fixture, "--min-item-chars", "0")
        self.assertEqual(out["error"]["code"], "VALIDATION.INPUT")

    def test_read_item_truncates_and_rejects_empty(self):
        long_item = write(self.d.name, "long.txt", "x" * 100)
        ns = jj.argparse.Namespace(item_file=long_item, item_id="bd-1", item_title="", max_item_chars=40)
        item = jj.read_item(ns)
        self.assertTrue(item["truncated"])
        self.assertTrue(item["text"].startswith("x" * 40) and item["text"].endswith(jj.TRUNCATION_MARKER))
        self.assertEqual(jj.item_core(item["text"]), "x" * 40)
        empty = write(self.d.name, "empty.txt", "  \n")
        with self.assertRaises(jj.JudgeError) as cm:
            jj.read_item(jj.argparse.Namespace(item_file=empty, item_id="x", item_title="", max_item_chars=10))
        self.assertEqual(cm.exception.code, "INPUT.EMPTY")
        with self.assertRaises(jj.JudgeError) as cm:
            jj.read_item(jj.argparse.Namespace(item_file=str(Path(self.d.name) / "nope.txt"), item_id="x", item_title="", max_item_chars=10))
        self.assertEqual(cm.exception.code, "INPUT.MISSING")

    def test_run_file_names_are_sanitised_and_never_collide(self):
        with env():
            run_a = jj.assemble_run({"gate": "plan", "model": "m", "threshold": 0.6, "max_request_bytes": 1, "min_item_chars": 1, "dry_run": False, "answers_fixture": None},
                                    record(), "a.json", {"id": "PR #12/x", "title": "", "text": "t", "truncated": False, "source": "s"}, [], [], None)
        self.assertTrue(run_a["run_id"].endswith("-PR__12_x"))
        run_dir = Path(self.d.name) / "coll"
        p1 = jj.write_run(dict(run_a), "", run_dir)
        run_b = dict(run_a)
        p2 = jj.write_run(run_b, "", run_dir)
        self.assertNotEqual(p1, p2)
        self.assertTrue(run_b["run_id"].endswith("-2") and p2.endswith("-2.json"))
        self.assertEqual(sorted(x.name for x in run_dir.iterdir() if x.suffix == ".tmp"), [])


class FailClosedTests(unittest.TestCase):
    def test_missing_key_and_missing_opt_in(self):
        with tempfile.TemporaryDirectory() as d:
            transport = Transport()
            j = Judged(d, transport, env_values={})
            self.assertEqual((j.code, j.out["error"]["code"]), (2, "AUTH.CREDENTIALS"))
            j = Judged(d, transport, env_values={"TYPESAFE_API_KEY": "sk-test"})
            self.assertEqual((j.code, j.out["error"]["code"]), (2, "POLICY.REMOTE_JUDGE_DISABLED"))
            self.assertEqual(transport.bodies, [])  # nothing left the machine
            j = Judged(d, transport, env_values={"TYPESAFE_API_KEY": "sk-test"}, extra_args=["--allow-remote"])
            self.assertEqual((j.code, j.result["verdict"]), (0, "satisfied"))
            self.assertEqual(len(transport.bodies), 1)

    def test_key_format_checked_and_never_echoed(self):
        with tempfile.TemporaryDirectory() as d:
            j = Judged(d, Transport(), env_values={"TYPESAFE_API_KEY": "sk-bad\nkey", "PLAN_ANCHOR_ALLOW_REMOTE_JUDGE": "1"})
            self.assertEqual((j.code, j.out["error"]["code"]), (2, "AUTH.CREDENTIALS"))
            self.assertNotIn("sk-bad", json.dumps(j.out))
            j = Judged(d, Transport(), env_values={"TYPESAFE_API_KEY": "  sk-ok  ", "PLAN_ANCHOR_ALLOW_REMOTE_JUDGE": "1"})
            self.assertEqual(j.code, 0)

    def test_fixture_and_dry_run_need_neither(self):
        with tempfile.TemporaryDirectory() as d:
            fixture = write(d, "answers.json", {"*": plan_answers()})
            j = Judged(d, Transport(), env_values={}, extra_args=["--answers-file", fixture])
            self.assertEqual(j.code, 0)
            j = Judged(d, Transport(), env_values={}, extra_args=["--dry-run"])
            self.assertEqual(j.code, 0)


class TransportPolicyTests(unittest.TestCase):
    def test_overflow_once_then_success_halves_the_item(self):
        text = "y" * 6000
        with tempfile.TemporaryDirectory() as d:
            t = Transport(http(422, "max_tokens_exceeded"))
            j = Judged(d, t, item_text=text)
        self.assertEqual((j.code, j.result["verdict"], j.result["attempts"]), (0, "satisfied", 2))
        self.assertTrue(j.result["item_shortened"])
        self.assertEqual(j.result["item_chars_sent"], 3000 + len(jj.TRUNCATION_MARKER))
        self.assertEqual(j.sleeps, [])
        sent = t.bodies[1]["state"]["item"]["text"]
        self.assertTrue(sent.startswith("y" * 3000) and sent.endswith(jj.TRUNCATION_MARKER))
        self.assertEqual(t.bodies[0]["state"]["item"]["text"], text)
        self.assertEqual(j.run["summary"]["shortened"], ["ANC-001"])
        self.assertEqual(j.run["item"]["chars"], 6000)  # the run records what the item was, the entry what was sent

    def test_overflow_down_to_the_floor_records_context_exceeded(self):
        with tempfile.TemporaryDirectory() as d:
            t = Transport(*[http(422, "max_tokens_exceeded")] * 10)
            j = Judged(d, t, item_text="y" * 5000, extra_args=["--min-item-chars", "2000"])
        self.assertEqual((j.code, j.result["verdict"]), (1, "error"))
        self.assertEqual(j.result["error"]["code"], "JEV.CONTEXT_EXCEEDED")
        self.assertIn("Shorten the evidence", j.result["error"]["suggested_action"])
        self.assertEqual(j.result["attempts"], 3)  # 5000, 2500, 2000 (floor), then give up
        self.assertEqual([len(jj.item_core(b["state"]["item"]["text"])) for b in t.bodies], [5000, 2500, 2000])
        self.assertTrue(j.result["item_shortened"])
        self.assertEqual(j.result["item_chars_sent"], 2000 + len(jj.TRUNCATION_MARKER))
        self.assertTrue(j.out["success"])  # the run envelope stays success; the anchor carries the error

    def test_preflight_cap_shortens_before_the_first_call(self):
        with tempfile.TemporaryDirectory() as d:
            t = Transport()
            j = Judged(d, t, item_text="z" * 20000, extra_args=["--max-request-bytes", "8000"])
        self.assertEqual((j.code, j.result["attempts"], j.result["item_shortened"]), (0, 1, True))
        self.assertLessEqual(len(jj.encode_body(t.bodies[0])), 8000)
        self.assertLessEqual(j.result["request_bytes"], 8000)
        self.assertLess(j.result["item_chars_sent"], 20000)

    def test_preflight_cap_impossible_makes_no_call(self):
        with tempfile.TemporaryDirectory() as d:
            t = Transport()
            j = Judged(d, t, item_text="z" * 3000, extra_args=["--max-request-bytes", "500"])
        self.assertEqual((j.result["verdict"], j.result["error"]["code"], j.result["attempts"]), ("error", "JEV.CONTEXT_EXCEEDED", 0))
        self.assertEqual(t.bodies, [])
        with tempfile.TemporaryDirectory() as d:
            j = Judged(d, Transport(), item_text="z" * 3000, extra_args=["--max-request-bytes", "500", "--dry-run"])
        self.assertEqual(j.result["error"]["code"], "JEV.CONTEXT_EXCEEDED")
        self.assertEqual(j.code, 0)

    def test_rate_limited_twice_then_success(self):
        with tempfile.TemporaryDirectory() as d:
            t = Transport(http(429, retry_after=3.5), http(429))
            j = Judged(d, t)
        self.assertEqual((j.code, j.result["verdict"], j.result["attempts"]), (0, "satisfied", 3))
        self.assertEqual(j.sleeps, [3.5, 2.0])
        self.assertEqual(len(t.bodies), 3)
        with tempfile.TemporaryDirectory() as d:
            j = Judged(d, Transport(http(529), jj.JudgeError("JEV.NETWORK", "down", recoverable=True)))
        self.assertEqual((j.result["attempts"], j.sleeps), (3, [1.0, 2.0]))

    def test_three_failures_record_an_error_verdict(self):
        with tempfile.TemporaryDirectory() as d:
            t = Transport(http(529), http(529), http(529), http(529))
            j = Judged(d, t)
        self.assertEqual((j.code, j.result["verdict"], j.result["attempts"]), (1, "error", 3))
        self.assertEqual(j.result["error"]["code"], "JEV.OVERLOADED")
        self.assertTrue(j.result["error"]["recoverable"])
        self.assertEqual(len(t.bodies), 3)
        self.assertEqual(j.sleeps, [1.0, 2.0])
        self.assertEqual(j.run["summary"]["errors"], ["ANC-001"])
        self.assertFalse(j.run["summary"]["pass"])
        with tempfile.TemporaryDirectory() as d:
            j = Judged(d, Transport(http(503), http(503), http(503)))
        self.assertEqual((j.result["verdict"], j.result["error"]["code"], j.result["attempts"]), ("error", "JEV.HTTP", 3))

    def test_invalid_response_is_retried_like_a_transport_failure(self):
        good = {"model": "jev-1.13.0", "answers": plan_answers()}
        with tempfile.TemporaryDirectory() as d:
            j = Judged(d, Transport({"answers": {}}, {"answers": plan_answers() | {"extra": answer("yes")}}, good))
        self.assertEqual((j.result["verdict"], j.result["attempts"], j.sleeps), ("satisfied", 3, [1.0, 2.0]))
        with tempfile.TemporaryDirectory() as d:
            j = Judged(d, Transport({"answers": {}}, {"answers": {}}, {"answers": {}}))
        self.assertEqual((j.result["verdict"], j.result["error"]["code"], j.result["attempts"]), ("error", "JEV.RESPONSE_INVALID", 3))

    def test_credential_and_validation_errors_are_not_retried(self):
        with tempfile.TemporaryDirectory() as d:
            t = Transport(http(401))
            j = Judged(d, t, anchors=[anchor(), anchor("ANC-002")])
        self.assertEqual((j.code, j.out["error"]["code"], len(t.bodies)), (2, "AUTH.CREDENTIALS", 1))
        with tempfile.TemporaryDirectory() as d:
            t = Transport(http(422, "questions.addressed.criteria must be an object"))
            j = Judged(d, t)
        self.assertEqual((j.result["verdict"], j.result["error"]["code"], j.result["attempts"], j.sleeps), ("error", "JEV.VALIDATION", 1, []))
        self.assertIn("questions.addressed.criteria", j.result["error"]["message"])

    def test_one_anchor_failing_does_not_lose_the_others(self):
        with tempfile.TemporaryDirectory() as d:
            t = Transport(http(529), http(529), http(529))  # first anchor exhausts; second anchor gets the default
            j = Judged(d, t, anchors=[anchor(), anchor("ANC-002")])
        verdicts = {r["anchor_id"]: r["verdict"] for r in j.run["results"]}
        self.assertEqual(verdicts, {"ANC-001": "error", "ANC-002": "satisfied"})
        self.assertEqual([r["verdict"] for r in j.run["results"]], ["satisfied", "error"])  # severity order: error sorts last

    def test_model_answered_is_recorded(self):
        with tempfile.TemporaryDirectory() as d:
            j = Judged(d, Transport({"model": "jev-1.13.0", "answers": plan_answers(), "usage": {"input_tokens": 5}}))
        self.assertEqual((j.run["model_answered"], j.run["model_requested"], j.result["usage"]), ("jev-1.13.0", jj.DEFAULT_MODEL, {"input_tokens": 5}))


class CheckFixturesTests(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.TemporaryDirectory()
        a = anchor(fixtures={"pass": ["Sprint 2 copies atm-core/crates/runtime into crates/runtime."],
                             "fail": ["Sprint 2 implements a new runtime crate from scratch.", "Sprint 2 defers the runtime."]})
        self.anchors = write(self.d.name, "anchors.json", record(a, anchor("ANC-002", status="proposed")))

    def tearDown(self):
        self.d.cleanup()

    def check(self, *extra):
        with env():
            return run_cli(["check-fixtures", "--anchors", self.anchors, "--gate", "plan", *extra])

    def test_expected_verdicts_by_fixture_key(self):
        fixture = write(self.d.name, "answers.json", {
            "ANC-001:pass:1": plan_answers(),
            "ANC-001:fail:1": plan_answers(comp="no", conf="yes"),
            "*": plan_answers(comp="no"),
        })
        code, out = self.check("--id", "ANC-001", "--answers-file", fixture)
        self.assertEqual(code, 0)
        checks = out["data"]["checks"]
        self.assertEqual([(c["anchor_id"], c["kind"], c["index"], c["verdict"], c["expected_ok"]) for c in checks],
                         [("ANC-001", "pass", 1, "satisfied", True), ("ANC-001", "fail", 1, "contradicted", True), ("ANC-001", "fail", 2, "not_addressed", True)])
        self.assertEqual((out["data"]["mismatches"], out["data"]["pass"], out["data"]["runs"], out["data"]["path"], out["data"]["run_path"]), ([], True, [], None, None))

    def test_mismatch_exits_one_and_runs_go_under_fixtures_dir(self):
        fixture = write(self.d.name, "answers.json", {"*": plan_answers()})  # everything satisfied: fail fixtures mismatch
        run_dir = str(Path(self.d.name) / "runs")
        code, out = self.check("--all", "--answers-file", fixture, "--run-dir", run_dir)
        self.assertEqual(code, 1)
        self.assertEqual(out["data"]["mismatches"], ["ANC-001:fail:1", "ANC-001:fail:2"])
        self.assertEqual(out["data"]["path"], str(Path(run_dir) / "fixtures"))
        self.assertEqual(out["data"]["run_path"], out["data"]["path"])
        self.assertEqual(len(out["data"]["runs"]), 3)
        self.assertTrue(all(Path(p).is_file() and Path(p).parent.name == "fixtures" for p in out["data"]["runs"]))
        self.assertTrue(all("fixture_ANC-001_" in Path(p).name for p in out["data"]["runs"]))
        self.assertEqual(out["data"]["checks"][0]["run_path"], out["data"]["runs"][0])

    def test_no_fixtures_and_dry_run(self):
        code, out = self.check("--id", "ANC-002", "--dry-run")
        self.assertEqual((code, out["error"]["code"]), (2, "FIXTURE.NONE"))
        code, out = self.check("--all", "--dry-run")
        self.assertEqual(code, 0)
        self.assertEqual(out["data"]["totals"]["requests"], 3)
        self.assertEqual(out["data"]["requests"][0]["state"]["item"]["id"], "fixture:ANC-001:pass:1")
        self.assertEqual(out["data"]["requests"][0]["state"]["item"]["title"], "fixture")
        self.assertIsNone(out["data"]["checks"][0]["expected_ok"])
        code, out = self.check("--dry-run")
        self.assertEqual(out["error"]["code"], "VALIDATION.INPUT")

    def test_live_check_uses_the_policy_and_opt_in(self):
        with env():
            code, out = self.check("--id", "ANC-001")
        self.assertEqual(out["error"]["code"], "AUTH.CREDENTIALS")
        t = Transport(http(429), {"model": "jev", "answers": plan_answers()}, {"model": "jev", "answers": plan_answers(comp="no")},
                      {"model": "jev", "answers": plan_answers(conf="yes")})
        with env(**LIVE_ENV), mock.patch.object(jj, "call_jev", t), mock.patch.object(jj, "_sleep", lambda s: None):
            code, out = run_cli(["check-fixtures", "--anchors", self.anchors, "--gate", "plan", "--id", "ANC-001"])
        self.assertEqual(code, 0)
        self.assertEqual([c["verdict"] for c in out["data"]["checks"]], ["satisfied", "not_addressed", "contradicted"])
        self.assertEqual(len(t.bodies), 4)


class RetireUpdateTests(unittest.TestCase):
    def test_retire(self):
        with tempfile.TemporaryDirectory() as d:
            path = write(d, "anchors.json", record(anchor(), anchor("ANC-002")))
            code, out = run_cli(["retire", "--anchors", path, "--id", "ANC-001", "--replaced-by", "ANC-002", "--reason", "reworded"])
            self.assertEqual((code, out["data"]["status"], out["data"]["version"]), (0, "retired", 2))
            rec = json.loads(Path(path).read_text())
            a = rec["anchors"][0]
            self.assertEqual(a["status"], "retired")
            self.assertEqual(a["history"][-1]["event"], "retired")
            self.assertEqual((a["history"][-1]["replaced_by"], a["history"][-1]["reason"]), ("ANC-002", "reworded"))
            code, out = run_cli(["retire", "--anchors", path, "--id", "ANC-001"])
            self.assertEqual((code, out["error"]["code"]), (2, "VALIDATION.INPUT"))
            code, out = run_cli(["retire", "--anchors", path, "--id", "ANC-002", "--replaced-by", "ANC-404"])
            self.assertEqual(out["error"]["code"], "ANCHOR.UNKNOWN")
            # retired anchors are skipped by --all and render still works
            self.assertEqual(jj.validate_record(rec), [])

    def test_update_fields_and_fixtures(self):
        with tempfile.TemporaryDirectory() as d:
            path = write(d, "anchors.json", record(anchor()))
            code, out = run_cli(["update", "--anchors", path, "--id", "ANC-001", "--scope", "runtime crates only",
                                 "--deterministic-check", "git log --follow crates/runtime", "--fixture-pass", "copies ATM", "--fixture-fail", "rewrites"])
            self.assertEqual(code, 0)
            self.assertEqual(sorted(out["data"]["changed"]), ["deterministic_check", "fixtures", "scope"])
            a = json.loads(Path(path).read_text())["anchors"][0]
            self.assertEqual((a["version"], a["scope"], a["fixtures"]), (2, "runtime crates only", {"pass": ["copies ATM"], "fail": ["rewrites"]}))
            self.assertEqual((a["history"][-1]["event"], sorted(a["history"][-1]["fields"])), ("updated", ["deterministic_check", "fixtures", "scope"]))
            self.assertEqual(a["statement"], STATEMENT)
            code, out = run_cli(["update", "--anchors", path, "--id", "ANC-001", "--clear-fixtures", "--fixture-pass", "only this", "--summary", "s"])
            a = json.loads(Path(path).read_text())["anchors"][0]
            self.assertEqual((a["fixtures"], a["version"]), ({"pass": ["only this"], "fail": []}, 3))
            code, out = run_cli(["update", "--anchors", path, "--id", "ANC-001", "--summary", "s"])  # unchanged value
            self.assertEqual((code, out["error"]["code"]), (2, "VALIDATION.INPUT"))
            code, out = run_cli(["update", "--anchors", path, "--id", "ANC-001", "--statement", "new words"])
            self.assertEqual((code, out["error"]["code"]), (2, "VALIDATION.INPUT"))
            self.assertIn("immutable", out["error"]["message"])

    def test_promote_and_set_beads(self):
        with tempfile.TemporaryDirectory() as d:
            path = write(d, "anchors.json", record(anchor()))
            code, out = run_cli(["set-beads", "--anchors", path, "--id", "ANC-001", "--anchor-bead", "bd-10", "--plan-bead", "bd-11"])
            self.assertEqual(out["data"]["beads"], {"anchor": "bd-10", "plan_questions": "bd-11", "dev_questions": None})
            code, out = run_cli(["promote", "--anchors", path, "--id", "ANC-001", "--adr", "ADR-0007"])
            a = json.loads(Path(path).read_text())["anchors"][0]
            self.assertEqual((a["status"], a["promoted_to"], a["version"]), ("promoted", "ADR-0007", 2))
            code, out = run_cli(["render", "--anchors", path])
            md = Path(out["data"]["path"]).read_text()
            self.assertIn(f"> {STATEMENT}", md)
            self.assertIn("promoted to ADR-0007", md)
            self.assertIn("Beads: anchor `bd-10`", md)


class ComposeItemTests(unittest.TestCase):
    SHOW = [{"id": "bd-42", "title": "Sprint 2 runtime", "description": "Copy the ATM crates.", "acceptance_criteria": "- builds\n- tests pass", "notes": ""}]

    def test_sections_in_order_and_empty_omitted(self):
        composed = jj.compose_item_text(self.SHOW)
        self.assertEqual(composed["sections"], ["Description", "Acceptance criteria"])
        self.assertEqual(composed["text"], "# Sprint 2 runtime\n\n## Description\n\nCopy the ATM crates.\n\n## Acceptance criteria\n\n- builds\n- tests pass\n")
        self.assertEqual((composed["item_id"], composed["chars"]), ("bd-42", len(composed["text"])))
        self.assertEqual(jj.compose_item_text(self.SHOW[0])["title"], "Sprint 2 runtime")  # bare object accepted
        with_design = dict(self.SHOW[0], design="Keep the trait object-safe.")
        self.assertEqual(jj.compose_item_text(with_design)["sections"], ["Description", "Design", "Acceptance criteria"])

    def test_closed_children_appended(self):
        children = [{"id": "bd-43", "title": "copy crates", "status": "closed", "close_reason": "done in 3f2a"},
                    {"id": "bd-44", "title": "open one", "status": "open"},
                    {"id": "bd-45", "title": "no reason", "status": "closed"}]
        composed = jj.compose_item_text(self.SHOW, children)
        self.assertEqual(composed["sections"][-1], "Closed beads")
        self.assertIn("## Closed beads\n\n- bd-43 copy crates: done in 3f2a\n- bd-45 no reason:\n", composed["text"])
        self.assertNotIn("bd-44", composed["text"])
        self.assertEqual(jj.compose_item_text(self.SHOW, [])["sections"], ["Description", "Acceptance criteria"])

    def test_cli_out_file_stdin_and_bad_shapes(self):
        with tempfile.TemporaryDirectory() as d:
            show = write(d, "show.json", self.SHOW)
            children = write(d, "children.json", [{"id": "bd-43", "title": "t", "status": "closed", "close_reason": "r"}])
            out_path = str(Path(d) / "item.txt")
            code, out = run_cli(["compose-item", "--show-json", show, "--children-json", children, "--out", out_path])
            self.assertEqual((code, out["data"]["path"], out["data"]["item_id"], out["data"]["title"]), (0, out_path, "bd-42", "Sprint 2 runtime"))
            self.assertEqual(out["data"]["sections"], ["Description", "Acceptance criteria", "Closed beads"])
            self.assertEqual(Path(out_path).read_text(), jj.compose_item_text(self.SHOW, json.loads(Path(children).read_text()))["text"])
            self.assertNotIn("text", out["data"])
            code, out = run_cli(["compose-item", "--show-json", show])
            self.assertIsNone(out["data"]["path"])
            self.assertTrue(out["data"]["text"].startswith("# Sprint 2 runtime"))
            with mock.patch.object(sys, "stdin", io.StringIO(json.dumps(self.SHOW))):
                code, out = run_cli(["compose-item", "--show-json", "-"])
            self.assertEqual(out["data"]["item_id"], "bd-42")
            two = write(d, "two.json", self.SHOW + self.SHOW)
            code, out = run_cli(["compose-item", "--show-json", two])
            self.assertEqual((code, out["error"]["code"]), (2, "INPUT.MALFORMED"))
            not_array = write(d, "obj.json", {"id": "bd-43"})
            code, out = run_cli(["compose-item", "--show-json", show, "--children-json", not_array])
            self.assertEqual((code, out["error"]["code"]), (2, "INPUT.MALFORMED"))  # children must be an array


def make_run(d, name, gate, item, created_at, verdicts, title=""):
    results = [{"anchor_id": aid, "source_type": "ANC" if aid.startswith("ANC") else aid[:3], "source_ref": None if aid.startswith("ANC") else aid,
                "statement": STATEMENT if aid.startswith("ANC") else "All public CLI commands must support --json output with a stable schema.",
                "stated_by": "Rand", "stated_at": "2026-10-09", "context": "planning session", "anchor_version": 1,
                "question_set": {"gate": gate, "version": 1, "overridden": False}, "deterministic_check": "git log --follow crates/runtime",
                "verdict": v, "answers": {}, "min_confidence": 0.9, "explanation": jj.explain(v, 0.9, gate) if v else "dry run"}
               for aid, v in verdicts.items()]
    for r in results:
        if r["verdict"] == "error":
            r["error"] = {"code": "JEV.OVERLOADED", "message": "Jev HTTP 529; response body withheld", "recoverable": True, "suggested_action": "Retry"}
    failing = [a for a, v in verdicts.items() if v in jj.FAILING]
    run = {"schema": "plan-anchoring/run/1", "run_id": f"{gate}-{created_at}-{item}", "gate": gate, "created_at": created_at,
           "anchors_path": "anchors.json", "plan": "phase-current",
           "item": {"id": item, "title": title, "source": "f", "chars": 10, "truncated": False},
           "model_requested": "jev-latest", "model_answered": "jev-1.13.0", "threshold": 0.6, "dry_run": False, "fixture": False,
           "results": results, "summary": {"counts": {}, "failing": failing, "errors": [a for a, v in verdicts.items() if v == "error"],
                                           "pass": not failing and "error" not in verdicts.values()}}
    return write(d, name, run)


class ReportTests(unittest.TestCase):
    def test_line_format_quotes_the_statement(self):
        with tempfile.TemporaryDirectory() as d:
            path = make_run(d, "r.json", "plan", "bd-42", "2026-10-09T18:30:00+00:00",
                            {"ANC-012": "not_addressed", "REQ-014": "contradicted", "ANC-013": "satisfied", "ANC-014": "not_applicable"}, title="Sprint 2 runtime")
            code, md = run_cli(["report", "--run", path, "--format", "md"])
            self.assertEqual(code, 0)
            self.assertIn("## Anchor gate: plan · bd-42 - Sprint 2 runtime", md)
            self.assertIn("Run `plan-2026-10-09T18:30:00+00:00-bd-42`, model `jev-1.13.0`, threshold 0.6.", md)
            self.assertIn(f'> **bd-42 · ANC-012 · not addressed** You said (planning session, Oct 9): "{STATEMENT}." This bead does not carry the directive forward.', md)
            self.assertIn('> **bd-42 · REQ-014 · contradicted** REQ-014 (requirements register) says: "All public CLI commands must support --json output with a stable schema." This bead proposes something the anchor rules out.', md)
            self.assertNotIn("schema.\"", md.replace('schema." This', ""))  # no double period
            self.assertNotIn("ANC-013", md)
            code, out = run_cli(["report", "--run", path])
            self.assertEqual((out["data"]["pass"], out["data"]["failing"]), (False, ["ANC-012", "REQ-014"]))
            self.assertEqual(out["data"]["markdown"], md.rstrip("\n") + "\n")  # print() adds one newline
            code, md_all = run_cli(["report", "--run", path, "--format", "md", "--all"])
            self.assertIn("> **bd-42 · ANC-013 · satisfied**", md_all)
            self.assertIn("> **bd-42 · ANC-014 · not applicable**", md_all)

    def test_register_names_dev_checks_and_errors(self):
        self.assertEqual(jj.quote_statement("ends with ?"), '"ends with ?"')
        self.assertEqual(jj.quote_statement(" plain "), '"plain."')
        with tempfile.TemporaryDirectory() as d:
            path = make_run(d, "r.json", "dev", "sprint-2", "2026-10-22T10:15:00+00:00", {"ANC-012": "asserted_only", "ADR-0007": "satisfied", "ANC-020": "error"})
            run = json.loads(Path(path).read_text())
            run["results"][1]["statement"] = "The CLI stays thin"
            Path(path).write_text(json.dumps(run))
            code, md = run_cli(["report", "--run", path, "--format", "md", "--all"])
            self.assertIn("> **sprint-2 · ANC-012 · asserted only** You said", md)
            self.assertIn(">   Pair with the deterministic check: git log --follow crates/runtime", md)
            self.assertIn('ADR-0007 (ADR register) says: "The CLI stays thin."', md)
            self.assertIn("> **sprint-2 · ANC-020 · error**", md)
            self.assertIn(">   Error JEV.OVERLOADED: Jev HTTP 529; response body withheld", md)
            self.assertEqual(code, 0)

    def test_all_passing_message_and_bad_inputs(self):
        with tempfile.TemporaryDirectory() as d:
            path = make_run(d, "r.json", "plan", "bd-1", "2026-10-09T18:30:00+00:00", {"ANC-001": "satisfied"})
            code, md = run_cli(["report", "--run", path, "--format", "md"])
            self.assertIn("All judged anchors are satisfied or not applicable.", md)
            code, out = run_cli(["report", "--run", ""])
            self.assertEqual((code, out["error"]["code"]), (2, "INPUT.MISSING"))
            code, out = run_cli(["report", "--run", d])
            self.assertEqual((code, out["error"]["code"]), (2, "INPUT.MISSING"))
            other = write(d, "other.json", {"schema": "something-else"})
            code, out = run_cli(["report", "--run", other, "--format", "md"])
            self.assertEqual((code, out["error"]["code"]), (2, "INPUT.MALFORMED"))


class CoverageTests(unittest.TestCase):
    def test_uncovered_latest_wins_and_dry_runs_ignored(self):
        with tempfile.TemporaryDirectory() as d:
            runs = Path(d) / "runs"
            make_run(runs, "a1.json", "plan", "bd-1", "2026-10-09T10:00:00+00:00", {"ANC-001": "satisfied", "ANC-002": "not_applicable", "ANC-003": "contradicted"})
            make_run(runs, "a2.json", "plan", "bd-1", "2026-10-09T11:00:00+00:00", {"ANC-001": "not_addressed", "ANC-002": "not_applicable", "ANC-003": "satisfied"})
            make_run(runs, "b.json", "plan", "bd-2", "2026-10-09T10:30:00+00:00", {"ANC-001": "satisfied", "ANC-002": "not_applicable", "ANC-003": "contradicted"})
            make_run(runs, "dev.json", "dev", "bd-2", "2026-10-09T12:00:00+00:00", {"ANC-002": "satisfied"})
            dry = json.loads(Path(make_run(runs, "dry.json", "plan", "bd-3", "2026-10-09T13:00:00+00:00", {"ANC-002": "satisfied"})).read_text())
            dry["dry_run"] = True
            write(runs, "dry.json", dry)
            write(runs, "junk.json", "{not json")
            write(runs, "other.json", {"schema": "x"})
            code, out = run_cli(["coverage", "--runs", str(runs), "--gate", "plan"])
            data = out["data"]
            self.assertEqual((code, data["items"], data["runs_considered"]), (0, ["bd-1", "bd-2"], 2))
            self.assertEqual(data["matrix"]["ANC-001"], {"bd-1": "not_addressed", "bd-2": "satisfied"})  # a2 (later) replaced a1
            self.assertEqual(data["uncovered_anchors"], ["ANC-002"])  # not_applicable everywhere is uncovered on purpose
            self.assertEqual(data["contradicted"], {"ANC-003": ["bd-2"]})
            code, md = run_cli(["coverage", "--runs", str(runs), "--gate", "plan", "--format", "md"])
            self.assertIn("| Anchor | bd-1 | bd-2 | Covered |", md)
            self.assertIn("| ANC-001 | not addressed | satisfied | yes |", md)
            self.assertIn("| ANC-002 | not applicable | not applicable | no |", md)
            self.assertIn("Uncovered anchors (no item satisfies them): ANC-002", md)
            code, out = run_cli(["coverage", "--runs", str(runs)])
            self.assertEqual(out["data"]["items"], ["dev:bd-2", "plan:bd-1", "plan:bd-2"])  # gates never overwrite each other
            self.assertEqual(out["data"]["matrix"]["ANC-002"]["dev:bd-2"], "satisfied")
            self.assertNotIn("ANC-002", out["data"]["uncovered_anchors"])
            code, out = run_cli(["coverage", "--runs", str(runs), "--gate", "dev", "--format", "json"])
            self.assertEqual(out["data"]["items"], ["bd-2"])
            code, out = run_cli(["coverage", "--runs", str(Path(d) / "nowhere")])
            self.assertEqual((code, out["error"]["code"]), (2, "RUNS.NONE"))


class DiffTests(unittest.TestCase):
    def test_regressions_added_removed(self):
        with tempfile.TemporaryDirectory() as d:
            prev = make_run(d, "p.json", "plan", "bd-1", "2026-10-09T10:00:00+00:00",
                            {"ANC-001": "satisfied", "ANC-002": "not_addressed", "ANC-003": "satisfied", "ANC-004": "contradicted", "ANC-005": "satisfied"})
            nxt = make_run(d, "n.json", "plan", "bd-1", "2026-10-09T11:00:00+00:00",
                           {"ANC-001": "contradicted", "ANC-002": "satisfied", "ANC-003": "error", "ANC-004": "error", "ANC-006": "satisfied"})
            code, out = run_cli(["diff", "--prev", prev, "--next", nxt])
            data = out["data"]
            self.assertEqual(code, 0)
            changes = {c["anchor_id"]: c for c in data["changed"]}
            self.assertFalse(changes["ANC-001"]["improved"])
            self.assertTrue(changes["ANC-002"]["improved"])
            self.assertFalse(changes["ANC-003"]["improved"])  # satisfied -> error is a regression, not an improvement
            self.assertFalse(changes["ANC-004"]["improved"])  # losing a verdict is never an improvement
            self.assertEqual(data["regressions"], ["ANC-001", "ANC-003", "ANC-004"])
            self.assertEqual(data["added"], [{"anchor_id": "ANC-006", "verdict": "satisfied"}])
            self.assertEqual(data["removed"], [{"anchor_id": "ANC-005", "verdict": "satisfied"}])
            self.assertEqual((data["pass_before"], data["pass_after"], data["item"]), (False, False, "bd-1"))

    def test_mismatched_runs_refused(self):
        with tempfile.TemporaryDirectory() as d:
            a = make_run(d, "a.json", "plan", "bd-1", "2026-10-09T10:00:00+00:00", {"ANC-001": "satisfied"})
            b = make_run(d, "b.json", "dev", "bd-1", "2026-10-09T10:00:00+00:00", {"ANC-001": "satisfied"})
            c = make_run(d, "c.json", "plan", "bd-2", "2026-10-09T10:00:00+00:00", {"ANC-001": "satisfied"})
            code, out = run_cli(["diff", "--prev", a, "--next", b])
            self.assertEqual((code, out["error"]["code"]), (2, "RUNS.MISMATCH"))
            code, out = run_cli(["diff", "--prev", a, "--next", c])
            self.assertEqual((code, out["error"]["code"]), (2, "RUNS.MISMATCH"))
            code, out = run_cli(["diff", "--prev", a, "--next", str(Path(d) / "missing.json")])
            self.assertEqual((code, out["error"]["code"]), (2, "INPUT.MISSING"))


class ErrorHandlingTests(unittest.TestCase):
    def test_load_json_rejects_non_files(self):
        with tempfile.TemporaryDirectory() as d:
            for bad in ("", d, str(Path(d) / "missing.json")):
                with self.assertRaises(jj.JudgeError) as cm:
                    jj.load_json(bad, "thing")
                self.assertEqual(cm.exception.code, "INPUT.MISSING", bad)
            bad_json = write(d, "bad.json", "{")
            with self.assertRaises(jj.JudgeError) as cm:
                jj.load_json(bad_json, "thing")
            self.assertEqual(cm.exception.code, "INPUT.MALFORMED")

    def test_every_file_argument_yields_json_errors(self):
        with tempfile.TemporaryDirectory() as d:
            missing = str(Path(d) / "none.json")
            for argv in (["validate", "--anchors", missing], ["render", "--anchors", missing], ["set-beads", "--anchors", missing, "--id", "x"],
                         ["promote", "--anchors", missing, "--id", "x", "--adr", "A"], ["retire", "--anchors", missing, "--id", "x"],
                         ["update", "--anchors", missing, "--id", "x", "--scope", "s"], ["report", "--run", missing],
                         ["diff", "--prev", missing, "--next", missing], ["compose-item", "--show-json", missing],
                         ["judge", "--anchors", missing, "--gate", "plan", "--all", "--item-id", "x", "--dry-run"],
                         ["check-fixtures", "--anchors", missing, "--gate", "plan", "--all", "--dry-run"]):
                code, out = run_cli(argv)
                self.assertEqual((code, out["error"]["code"]), (2, "INPUT.MISSING"), argv)
            not_record = write(d, "list.json", [1, 2])
            code, out = run_cli(["validate", "--anchors", not_record])
            self.assertEqual(out["error"]["code"], "INPUT.MALFORMED")

    def test_missing_item_file_is_json_not_traceback(self):
        with tempfile.TemporaryDirectory() as d:
            anchors = write(d, "anchors.json", record(anchor()))
            with env():
                code, out = run_cli(["judge", "--anchors", anchors, "--gate", "plan", "--all", "--item-id", "x", "--item-file", str(Path(d) / "nope.txt"), "--dry-run"])
            self.assertEqual((code, out["error"]["code"]), (2, "INPUT.MISSING"))

    def test_unexpected_exceptions_become_script_internal(self):
        with mock.patch.object(jj, "cmd_validate", side_effect=RuntimeError("boom")):
            code, out = run_cli(["validate", "--anchors", "x"])
        self.assertEqual((code, out["error"]["code"], out["error"]["message"]), (2, "SCRIPT.INTERNAL", "RuntimeError: boom"))
        with mock.patch.object(jj, "cmd_validate", side_effect=KeyboardInterrupt):
            code, out = run_cli(["validate", "--anchors", "x"])
        self.assertEqual((code, out["canceled"]), (130, True))

    def test_env_numbers_are_checked(self):
        with env(PLAN_ANCHOR_CONFIDENCE_THRESHOLD="high"):
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf), self.assertRaises(SystemExit) as cm:
                jj._env_number("PLAN_ANCHOR_CONFIDENCE_THRESHOLD", 0.6, float)
            self.assertEqual(cm.exception.code, 2)
            self.assertEqual(json.loads(buf.getvalue())["error"]["code"], "VALIDATION.INPUT")
        with env(PLAN_ANCHOR_CONFIDENCE_THRESHOLD="0.7"):
            self.assertEqual(jj._env_number("PLAN_ANCHOR_CONFIDENCE_THRESHOLD", 0.6, float), 0.7)
        with env():
            self.assertEqual(jj._env_number("PLAN_ANCHOR_CONFIDENCE_THRESHOLD", 0.6, float), 0.6)

    def test_exit_codes(self):
        self.assertEqual(jj.exit_code_for("judge", {"success": True, "data": {"dry_run": False, "summary": {"pass": True}}}), 0)
        self.assertEqual(jj.exit_code_for("judge", {"success": True, "data": {"dry_run": False, "summary": {"pass": False}}}), 1)
        self.assertEqual(jj.exit_code_for("judge", {"success": True, "data": {"dry_run": True, "summary": {"pass": False}}}), 0)
        self.assertEqual(jj.exit_code_for("check-fixtures", {"success": True, "data": {"mismatches": ["a"]}}), 1)
        self.assertEqual(jj.exit_code_for("check-fixtures", {"success": True, "data": {"mismatches": []}}), 0)
        self.assertEqual(jj.exit_code_for("validate", {"success": False, "data": {}}), 1)
        self.assertEqual(jj.exit_code_for("report", "markdown text"), 0)

    def test_documented_question_sets_load(self):
        for gate in ("plan", "dev"):
            qs = jj.load_question_set(gate)
            self.assertEqual(qs["gate"], gate)
            for q in qs["questions"].values():
                self.assertEqual(set(q["criteria"]), set(OPTIONS))
        self.assertEqual(set(jj.load_question_set("dev")["roles"]), {"relevance", "compliance", "conflict", "evidence"})
        with self.assertRaises(jj.JudgeError) as cm:
            jj.load_question_set("plan", str(Path(__file__)))
        self.assertEqual(cm.exception.code, "INPUT.MALFORMED")


if __name__ == "__main__":
    unittest.main()
