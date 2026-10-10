#!/usr/bin/env python3
"""jev_judge.py - the deterministic half of plan anchoring.

Everything that must not be left to a language model lives here: anchor ID
allocation, the exact Jev request, turning Jev's independent answers into one
verdict per (anchor, item) pair, coverage counting across items, run-to-run
diffs, and the human report that quotes the anchor's verbatim statement.

Standard library only. Python 3.9+.

Subcommands
  add             append an anchor record (allocates the next ANC-NNN)
  update          edit scope / summary / deterministic_check / fixtures (never the statement)
  retire          mark an anchor retired, optionally naming its replacement
  validate        check anchors.json for structural problems (plus non-fatal warnings)
  render          write anchors.md from anchors.json
  set-beads       record the anchor / plan-questions / dev-questions bead IDs
  promote         mark an anchor as promoted to an ADR (keeps its ID)
  compose-item    build the labelled item text from `bd show --json` (and `bd children --json`)
  judge           ask Jev about one item against one or more anchors, write a run
  check-fixtures  judge each anchor's fixtures.pass / fixtures.fail sentences as items
  report          render a run as markdown in the provenance-quoting format
  coverage        anchors x items verdict matrix over one or more runs
  diff            what changed between two runs of the same gate and item

Every subcommand prints JSON to stdout (or markdown with --format md where
supported).

Exit codes
  0  success; for `judge`, the gate passed (or it was a dry run); for
     `check-fixtures`, every fixture landed on its expected verdict
  1  the command ran but the answer is "no": `judge` has a failing or error
     verdict, `check-fixtures` has a mismatch, `validate` found problems
  2  fatal: a JSON error object was printed (missing input, bad credentials,
     policy opt-in missing, malformed record or override, Jev unreachable
     before any verdict, or an internal error as SCRIPT.INTERNAL)
  130 interrupted by the user
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import http.client
import json
import math
import os
import pathlib
import re
import sys
import time
import urllib.error
import urllib.request


def _env_number(name: str, default, cast):
    """Read a numeric environment variable; a non-numeric value is a JSON error, not a traceback."""
    raw = os.environ.get(name)
    if raw is None or not raw.strip():
        return default
    try:
        return cast(raw)
    except ValueError:
        print(json.dumps({"success": False, "data": None, "error": {
            "code": "VALIDATION.INPUT", "message": f"{name}={raw!r} is not a number",
            "recoverable": False, "suggested_action": f"Unset {name} or give it a numeric value."}}, indent=2))
        sys.exit(2)


JEV_URL = os.environ.get("PLAN_ANCHOR_JEV_URL", "https://api.typesafe.ai/v1/systemone")
DEFAULT_MODEL = os.environ.get("PLAN_ANCHOR_JEV_MODEL", "jev-latest")
DEFAULT_THRESHOLD = _env_number("PLAN_ANCHOR_CONFIDENCE_THRESHOLD", 0.6, float)
DEFAULT_MAX_ITEM_CHARS = _env_number("PLAN_ANCHOR_MAX_ITEM_CHARS", 24000, int)
DEFAULT_MAX_REQUEST_BYTES = 24000  # conservative transport bound per request, as in process-porn's client
DEFAULT_MIN_ITEM_CHARS = 2000      # below this the item is not shortened further on a context overflow
MAX_RESPONSE_BYTES = 1048576
MAX_ATTEMPTS = 3                   # per anchor, for transport-level failures (429, 529, 5xx, network, bad response)
RETRY_DELAYS = (1.0, 2.0)          # seconds before the second and third attempt; Retry-After overrides the first
MAX_RETRY_AFTER = 30.0
PROBABILITY_ROUNDING = 0.01        # Jev rounds each probability; the sum may be off by this per option
TRUNCATION_MARKER = "\n[... truncated by jev_judge; keep the state small ...]"
HERE = pathlib.Path(__file__).resolve().parent
QUESTIONS_DIR = HERE.parent / "assets" / "questions"

SOURCE_TYPES = ("ANC", "REQ", "ADR")
REGISTER_NAMES = {"REQ": "requirements register", "ADR": "ADR register"}
STATUSES = ("proposed", "locked", "promoted", "retired")
JUDGED_STATUSES = ("locked", "promoted")
VERDICTS = (
    "contradicted",
    "not_addressed",
    "asserted_only",
    "underspecified",
    "satisfied",
    "not_applicable",
    "error",
)
FAILING = {"contradicted", "not_addressed", "asserted_only", "underspecified"}
SEVERITY = {v: i for i, v in enumerate(VERDICTS)}
REQUIRED_ROLES = ("relevance", "compliance", "conflict")
ABORT_CODES = ("AUTH.CREDENTIALS", "POLICY.REMOTE_JUDGE_DISABLED")
TRANSIENT_CODES = ("JEV.RATE_LIMITED", "JEV.OVERLOADED", "JEV.NETWORK", "JEV.RESPONSE_INVALID")
FIXTURE_EXPECTED = {"pass": ("satisfied",), "fail": ("not_addressed", "contradicted")}
ITEM_SECTIONS = (("description", "Description"), ("design", "Design"), ("acceptance_criteria", "Acceptance criteria"), ("notes", "Notes"))

_sleep = time.sleep  # replaced in tests


# --------------------------------------------------------------------------- #
# plumbing
# --------------------------------------------------------------------------- #
class JudgeError(Exception):
    def __init__(self, code: str, message: str, recoverable: bool = False, action: str = "",
                 status=None, retry_after=None):
        super().__init__(message)
        self.code = code
        self.recoverable = recoverable
        self.action = action
        self.status = status            # HTTP status when the error came from Jev
        self.retry_after = retry_after  # parsed Retry-After header, seconds, or None
        self.meta = {}                  # attempt / shortening bookkeeping, filled by judge_with_policy

    def to_json(self) -> dict:
        return {
            "success": False,
            "data": None,
            "error": {
                "code": self.code,
                "message": str(self),
                "recoverable": self.recoverable,
                "suggested_action": self.action,
            },
        }


def emit(obj, fmt: str = "json") -> None:
    if fmt == "json":
        print(json.dumps(obj, indent=2, ensure_ascii=False))
    else:
        print(obj)


def now_utc() -> _dt.datetime:
    return _dt.datetime.now(_dt.timezone.utc)


def stamp() -> str:
    return now_utc().strftime("%Y%m%dT%H%M%SZ")


def history_event(event: str, **fields) -> dict:
    ev = {"at": now_utc().isoformat(timespec="seconds"), "event": event}
    ev.update(fields)
    return ev


def load_json(path: str, what: str):
    if path == "-":
        raw = sys.stdin.read()
        try:
            return json.loads(raw)
        except json.JSONDecodeError as exc:
            raise JudgeError("INPUT.MALFORMED", f"{what} on stdin is not valid JSON: {exc}")
    p = pathlib.Path(path) if path else None
    if p is None or not path.strip() or not p.is_file():
        raise JudgeError("INPUT.MISSING", f"{what} not found: {path!r}" + (" (not a regular file)" if p is not None and path.strip() and p.exists() else ""),
                         action=f"Pass the path of an existing file for the {what}.")
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise JudgeError("INPUT.MALFORMED", f"{what} is not valid JSON: {exc}", action=f"Fix the JSON in {path}.")
    except (OSError, UnicodeDecodeError) as exc:
        raise JudgeError("INPUT.MALFORMED", f"{what} could not be read: {exc}", action=f"Check {path} is a readable UTF-8 file.")


def load_record(path: str) -> dict:
    rec = load_json(path, "anchors.json")
    if not isinstance(rec, dict):
        raise JudgeError("INPUT.MALFORMED", "anchors.json must be a JSON object", action="See references/anchor-document.md for the schema.")
    return rec


def load_run(path: str, what: str) -> dict:
    run = load_json(path, what)
    if not isinstance(run, dict) or run.get("schema") != "plan-anchoring/run/1" or not isinstance(run.get("results"), list) \
            or not isinstance(run.get("item"), dict) or not isinstance(run.get("summary"), dict):
        raise JudgeError("INPUT.MALFORMED", f"{what} {path} is not a plan-anchoring run file", action="Pass a file written by `judge`.")
    return run


def write_text_atomic(path: str, text: str) -> None:
    p = pathlib.Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    tmp = p.with_name(p.name + ".tmp")
    tmp.write_text(text, encoding="utf-8")
    tmp.replace(p)


def save_json(path: str, obj: dict) -> None:
    write_text_atomic(path, json.dumps(obj, indent=2, ensure_ascii=False) + "\n")


def short_date(iso: str) -> str:
    try:
        d = _dt.date.fromisoformat(iso[:10])
        return d.strftime("%b %-d") if os.name != "nt" else d.strftime("%b %d").replace(" 0", " ")
    except (ValueError, TypeError):
        return iso or "undated"


def sanitize_id(text: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]", "_", text)


# --------------------------------------------------------------------------- #
# anchor records
# --------------------------------------------------------------------------- #
def empty_record(plan: str, intent_ref: str) -> dict:
    return {
        "schema": "plan-anchoring/anchors/1",
        "plan": plan,
        "intent_ref": intent_ref,
        "anchors": [],
    }


def anchor_list(rec: dict) -> list:
    anchors = rec.get("anchors")
    if anchors is None:
        anchors = rec["anchors"] = []
    if not isinstance(anchors, list):
        raise JudgeError("INPUT.MALFORMED", "anchors.json: 'anchors' must be a list", action="Fix the record by hand, then run validate.")
    return anchors


def find_anchor(rec: dict, anchor_id: str) -> dict:
    for a in anchor_list(rec):
        if isinstance(a, dict) and a.get("id") == anchor_id:
            return a
    raise JudgeError("ANCHOR.UNKNOWN", f"No anchor with id {anchor_id}", action="Check anchors.json for the exact ID.")


def next_anc_id(rec: dict) -> str:
    nums = []
    for a in anchor_list(rec):
        aid = a.get("id") if isinstance(a, dict) else None
        if isinstance(aid, str) and re.fullmatch(r"ANC-\d{3,}", aid):
            nums.append(int(aid[4:]))
    return f"ANC-{(max(nums) + 1 if nums else 1):03d}"


def require_valid(rec: dict) -> None:
    problems = validate_record(rec)
    if problems:
        raise JudgeError("VALIDATION.ANCHORS", "anchors.json has problems: " + "; ".join(problems[:5]), recoverable=True,
                         action="Run `jev_judge.py validate` and fix the record first.")


def cmd_add(args) -> dict:
    p = pathlib.Path(args.anchors)
    rec = load_record(args.anchors) if p.exists() else empty_record(args.plan, args.intent_ref)
    anchors = anchor_list(rec)
    statement = args.statement.strip()
    if not statement:
        raise JudgeError("VALIDATION.INPUT", "statement must be the user's verbatim words and cannot be empty")
    if args.status == "retired":
        raise JudgeError("VALIDATION.INPUT", "a new anchor cannot be added as retired", action="Add it as proposed or locked; retire it later with `retire`.")
    if args.source_type == "ANC":
        anchor_id = next_anc_id(rec)
        source_ref = None
    else:
        if not args.source_ref:
            raise JudgeError("VALIDATION.INPUT", f"--source-ref is required for source type {args.source_type}",
                             action="Pass the register ID, e.g. --source-ref REQ-014.")
        anchor_id = args.source_ref
        source_ref = args.source_ref
        if any(isinstance(a, dict) and a.get("id") == anchor_id for a in anchors):
            raise JudgeError("ANCHOR.DUPLICATE", f"{anchor_id} already recorded")
    record = {
        "id": anchor_id,
        "source_type": args.source_type,
        "source_ref": source_ref,
        "statement": statement,
        "stated_by": args.stated_by,
        "stated_at": args.stated_at or _dt.date.today().isoformat(),
        "context": args.context,
        "summary": args.summary or "",
        "scope": args.scope or "",
        "deterministic_check": args.deterministic_check or "",
        "fixtures": {"pass": [], "fail": []},
        "question_overrides": {},
        "version": 1,
        "status": args.status,
        "promoted_to": None,
        "beads": {"anchor": None, "plan_questions": None, "dev_questions": None},
        "history": [history_event("created", status=args.status)],
    }
    anchors.append(record)
    save_json(args.anchors, rec)
    return {"success": True, "data": {"anchor": record, "path": args.anchors}, "error": None}


def cmd_update(args) -> dict:
    if args.statement is not None:
        raise JudgeError("VALIDATION.INPUT", "the statement is immutable; a changed directive is a new anchor",
                         action="Run `retire --id <old> --replaced-by <new>` after `add`ing the new wording.")
    rec = load_record(args.anchors)
    a = find_anchor(rec, args.id)
    changed = []
    for key, val in (("scope", args.scope), ("summary", args.summary), ("deterministic_check", args.deterministic_check)):
        if val is not None and (a.get(key) or "") != val:
            a[key] = val
            changed.append(key)
    fixtures = a.get("fixtures") if isinstance(a.get("fixtures"), dict) else {}
    before = json.dumps(fixtures, sort_keys=True)
    if args.clear_fixtures:
        fixtures = {"pass": [], "fail": []}
    fixtures.setdefault("pass", [])
    fixtures.setdefault("fail", [])
    for s in args.fixture_pass or []:
        if s.strip():
            fixtures["pass"].append(s.strip())
    for s in args.fixture_fail or []:
        if s.strip():
            fixtures["fail"].append(s.strip())
    if json.dumps(fixtures, sort_keys=True) != before:
        a["fixtures"] = fixtures
        changed.append("fixtures")
    if not changed:
        raise JudgeError("VALIDATION.INPUT", "nothing to update", action="Pass --scope, --summary, --deterministic-check, --fixture-pass, --fixture-fail or --clear-fixtures.")
    a["version"] = int(a.get("version", 1)) + 1
    a.setdefault("history", []).append(history_event("updated", fields=changed))
    save_json(args.anchors, rec)
    return {"success": True, "data": {"id": a["id"], "changed": changed, "version": a["version"], "anchor": a}, "error": None}


def cmd_retire(args) -> dict:
    rec = load_record(args.anchors)
    a = find_anchor(rec, args.id)
    if a.get("status") == "retired":
        raise JudgeError("VALIDATION.INPUT", f"{args.id} is already retired")
    if args.replaced_by:
        find_anchor(rec, args.replaced_by)  # the replacement must exist
    a["status"] = "retired"
    a["version"] = int(a.get("version", 1)) + 1
    a.setdefault("history", []).append(history_event("retired", replaced_by=args.replaced_by or None, reason=args.reason or ""))
    save_json(args.anchors, rec)
    return {"success": True, "data": {"id": a["id"], "status": "retired", "replaced_by": args.replaced_by or None, "version": a["version"]}, "error": None}


def validate_record(rec: dict) -> list:
    problems = []
    if not isinstance(rec, dict):
        return ["anchors.json must be a JSON object"]
    if rec.get("schema") != "plan-anchoring/anchors/1":
        problems.append("schema must be 'plan-anchoring/anchors/1'")
    anchors = rec.get("anchors", [])
    if not isinstance(anchors, list):
        problems.append("'anchors' must be a list")
        return problems
    seen = set()
    for n, a in enumerate(anchors):
        if not isinstance(a, dict):
            problems.append(f"anchors[{n}]: must be an object")
            continue
        aid = a.get("id", "<missing>")
        if not isinstance(aid, str) or not aid:
            problems.append(f"anchors[{n}]: id must be a non-empty string")
            aid = str(aid)
        if aid in seen:
            problems.append(f"{aid}: duplicate id")
        seen.add(aid)
        if a.get("source_type") not in SOURCE_TYPES:
            problems.append(f"{aid}: source_type must be one of {SOURCE_TYPES}")
        if a.get("source_type") == "ANC" and not re.fullmatch(r"ANC-\d{3,}", aid):
            problems.append(f"{aid}: ANC ids look like ANC-001")
        if a.get("source_type") in ("REQ", "ADR") and not a.get("source_ref"):
            problems.append(f"{aid}: REQ/ADR anchors need source_ref")
        if not isinstance(a.get("statement"), str) or not a["statement"].strip():
            problems.append(f"{aid}: statement (verbatim) is empty")
        if a.get("status") not in STATUSES:
            problems.append(f"{aid}: status must be one of {STATUSES}")
        if a.get("status") == "promoted" and not a.get("promoted_to"):
            problems.append(f"{aid}: promoted anchors need promoted_to")
        if not a.get("stated_at"):
            problems.append(f"{aid}: stated_at missing")
        fx = a.get("fixtures")
        if fx is not None and (not isinstance(fx, dict) or not all(isinstance(fx.get(k, []), list) for k in ("pass", "fail"))):
            problems.append(f"{aid}: fixtures must be an object with 'pass' and 'fail' lists")
        overrides = a.get("question_overrides") or {}
        if not isinstance(overrides, dict):
            problems.append(f"{aid}: question_overrides must be an object")
            overrides = {}
        for gate, q in overrides.items():
            if gate not in ("plan", "dev"):
                problems.append(f"{aid}: question_overrides key must be plan or dev, got {gate}")
            elif not isinstance(q, dict) or "questions" not in q:
                problems.append(f"{aid}: question_overrides.{gate} must have a 'questions' object")
            elif not isinstance(q["questions"], dict):
                problems.append(f"{aid}: question_overrides.{gate}.questions must be an object")
    return problems


def record_warnings(rec: dict, anchors_path: str) -> list:
    warnings = []
    intent = rec.get("intent_ref")
    if intent:
        candidates = [pathlib.Path(intent), pathlib.Path(anchors_path).parent / pathlib.Path(intent).name]
        if not any(c.is_file() for c in candidates):
            warnings.append(f"intent_ref file not found: {intent}")
    anchors = rec.get("anchors") if isinstance(rec.get("anchors"), list) else []
    if not any(isinstance(a, dict) and a.get("status") in JUDGED_STATUSES for a in anchors):
        warnings.append("no locked anchors")
    return warnings


def cmd_validate(args) -> dict:
    rec = load_record(args.anchors)
    problems = validate_record(rec)
    warnings = record_warnings(rec, args.anchors)
    if problems:
        return {"success": False, "data": {"problems": problems, "warnings": warnings}, "error": {
            "code": "VALIDATION.ANCHORS", "message": f"{len(problems)} problem(s) in {args.anchors}",
            "recoverable": True, "suggested_action": "Fix the listed problems and re-run validate."}}
    return {"success": True, "data": {"anchors": len(rec["anchors"]), "path": args.anchors, "warnings": warnings}, "error": None}


def render_md(rec: dict) -> str:
    lines = [f"# Anchors - {rec.get('plan', 'plan')}", ""]
    lines.append("Facts stated during planning, kept verbatim. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.")
    lines.append("")
    if rec.get("intent_ref"):
        lines.append(f"Intent: `{rec['intent_ref']}`")
        lines.append("")
    lines.append("| ID | Status | Summary | Stated |")
    lines.append("| --- | --- | --- | --- |")
    for a in rec.get("anchors", []):
        lines.append(f"| {a['id']} | {a['status']} | {a.get('summary') or ''} | {a.get('stated_at', '')} |")
    lines.append("")
    for a in rec.get("anchors", []):
        lines.append(f"## {a['id']}" + (f" ({a['source_ref']})" if a.get("source_ref") and a['source_ref'] != a['id'] else ""))
        lines.append("")
        lines.append(f"> {a['statement']}")
        lines.append("")
        who = a.get("stated_by") or "user"
        lines.append(f"Said by {who}, {a.get('context') or 'planning'}, {a.get('stated_at', '')}. Status: {a['status']}" +
                     (f", promoted to {a['promoted_to']}" if a.get("promoted_to") else "") + f". Version {a.get('version', 1)}.")
        if a.get("summary"):
            lines.append("")
            lines.append(f"Summary: {a['summary']}")
        if a.get("scope"):
            lines.append("")
            lines.append(f"Scope: {a['scope']}")
        if a.get("deterministic_check"):
            lines.append("")
            lines.append(f"Deterministic check to pair with the development gate: {a['deterministic_check']}")
        b = a.get("beads") or {}
        if any(b.values()):
            lines.append("")
            lines.append(f"Beads: anchor `{b.get('anchor')}`, plan questions `{b.get('plan_questions')}`, dev questions `{b.get('dev_questions')}`")
        fx = a.get("fixtures") or {}
        if fx.get("pass") or fx.get("fail"):
            lines.append("")
            lines.append("Fixtures:")
            for s in fx.get("pass", []):
                lines.append(f"- pass: {s}")
            for s in fx.get("fail", []):
                lines.append(f"- fail: {s}")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def cmd_render(args) -> dict:
    rec = load_record(args.anchors)
    require_valid(rec)
    md = render_md(rec)
    out = args.out or str(pathlib.Path(args.anchors).with_name("anchors.md"))
    write_text_atomic(out, md)
    return {"success": True, "data": {"path": out, "anchors": len(rec["anchors"])}, "error": None}


def cmd_set_beads(args) -> dict:
    rec = load_record(args.anchors)
    a = find_anchor(rec, args.id)
    beads = a.setdefault("beads", {})
    if not isinstance(beads, dict):
        beads = a["beads"] = {}
    for key, val in (("anchor", args.anchor_bead), ("plan_questions", args.plan_bead), ("dev_questions", args.dev_bead)):
        if val:
            beads[key] = val
    a.setdefault("history", []).append(history_event("beads_set", beads=beads))
    save_json(args.anchors, rec)
    return {"success": True, "data": {"id": a["id"], "beads": beads}, "error": None}


def cmd_promote(args) -> dict:
    rec = load_record(args.anchors)
    a = find_anchor(rec, args.id)
    a["status"] = "promoted"
    a["promoted_to"] = args.adr
    a["version"] = int(a.get("version", 1)) + 1
    a.setdefault("history", []).append(history_event("promoted", to=args.adr))
    save_json(args.anchors, rec)
    return {"success": True, "data": {"id": a["id"], "promoted_to": args.adr, "version": a["version"]}, "error": None}


# --------------------------------------------------------------------------- #
# item composition (from bd JSON; the script never runs bd)
# --------------------------------------------------------------------------- #
def compose_item_text(show, children=None) -> dict:
    if isinstance(show, list):
        if len(show) != 1 or not isinstance(show[0], dict):
            raise JudgeError("INPUT.MALFORMED", "bd show --json output must be a one-element array holding the bead object",
                             action="Pass the unmodified output of `bd show <id> --json`.")
        show = show[0]
    if not isinstance(show, dict):
        raise JudgeError("INPUT.MALFORMED", "bead JSON must be an object or a one-element array")
    item_id = str(show.get("id") or "")
    title = str(show.get("title") or item_id or "untitled").strip()
    parts = [f"# {title}"]
    sections = []
    for key, heading in ITEM_SECTIONS:
        val = show.get(key)
        if isinstance(val, str) and val.strip():
            parts.append(f"## {heading}\n\n{val.strip()}")
            sections.append(heading)
    if children is not None:
        if not isinstance(children, list):
            raise JudgeError("INPUT.MALFORMED", "bd children --json output must be an array", action="Pass the unmodified output of `bd children <id> --json`.")
        closed = [c for c in children if isinstance(c, dict) and c.get("status") == "closed"]
        if closed:
            rows = [f"- {c.get('id', '?')} {str(c.get('title') or '').strip()}: {str(c.get('close_reason') or '').strip()}".rstrip() for c in closed]
            parts.append("## Closed beads\n\n" + "\n".join(rows))
            sections.append("Closed beads")
    text = "\n\n".join(parts) + "\n"
    return {"item_id": item_id, "title": title, "text": text, "sections": sections, "chars": len(text)}


def cmd_compose_item(args) -> dict:
    show = load_json(args.show_json, "bd show output")
    children = load_json(args.children_json, "bd children output") if args.children_json else None
    composed = compose_item_text(show, children)
    data = {"path": None, "item_id": composed["item_id"], "title": composed["title"], "sections": composed["sections"], "chars": composed["chars"]}
    if args.out:
        write_text_atomic(args.out, composed["text"])
        data["path"] = args.out
    else:
        data["text"] = composed["text"]
    return {"success": True, "data": data, "error": None}


# --------------------------------------------------------------------------- #
# judging
# --------------------------------------------------------------------------- #
def check_roles(qs: dict, where: str) -> None:
    """Every required role, and the optional evidence role if present, must name a question in the set."""
    questions = qs.get("questions")
    if not isinstance(questions, dict) or not questions:
        raise JudgeError("INPUT.MALFORMED", f"{where}: 'questions' must be a non-empty object keyed by question id",
                         action="See assets/questions/plan.json for the shape.")
    roles = qs.get("roles")
    if not isinstance(roles, dict):
        raise JudgeError("QUESTIONS.ROLES", f"{where}: 'roles' must map relevance, compliance and conflict to question ids",
                         action="See references/verdict-gating.md for the role contract.")
    for required in REQUIRED_ROLES:
        if roles.get(required) not in questions:
            raise JudgeError("QUESTIONS.ROLES", f"{where} must map role '{required}' to a question id in the set",
                             action="See references/verdict-gating.md for the role contract.")
    evidence = roles.get("evidence")
    if evidence is not None and evidence not in questions:
        raise JudgeError("QUESTIONS.ROLES", f"{where} maps role 'evidence' to '{evidence}', which is not a question in the set",
                         action="Keep the default question ids, or update roles to match the renamed question.")


def load_question_set(gate: str, override_path: str = "") -> dict:
    path = override_path or str(QUESTIONS_DIR / f"{gate}.json")
    qs = load_json(path, f"question set for gate '{gate}'")
    if not isinstance(qs, dict):
        raise JudgeError("INPUT.MALFORMED", f"question set {path} must be a JSON object")
    check_roles(qs, f"question set {path}")
    return qs


def merge_overrides(base: dict, override: dict) -> dict:
    """Per-anchor overrides replace question text by id; roles stay as in base unless overridden."""
    merged = json.loads(json.dumps(base))
    if not isinstance(override, dict):
        raise JudgeError("INPUT.MALFORMED", "a question override must be an object with 'questions' and optional 'roles'")
    questions = override.get("questions") or {}
    if not isinstance(questions, dict):
        raise JudgeError("INPUT.MALFORMED", "override 'questions' must be an object keyed by question id")
    for qid, q in questions.items():
        merged["questions"][qid] = q
    roles = override.get("roles")
    if roles:
        if not isinstance(roles, dict):
            raise JudgeError("INPUT.MALFORMED", "override 'roles' must be an object")
        merged.setdefault("roles", {}).update(roles)
    return merged


def questions_digest(questions: dict) -> str:
    canonical = json.dumps(questions, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


def build_state(anchor: dict, item: dict) -> dict:
    return {
        "anchor": {
            "id": anchor["id"],
            "statement": anchor["statement"],
            "scope": anchor.get("scope") or "",
        },
        "item": {
            "id": item["id"],
            "title": item.get("title") or "",
            "text": item["text"],
        },
    }


def encode_body(body: dict) -> bytes:
    return json.dumps(body, allow_nan=False).encode("utf-8")


def parse_retry_after(value):
    if value is None:
        return None
    try:
        delay = float(value)
    except (TypeError, ValueError):
        return None
    if not math.isfinite(delay) or delay < 0:
        return None
    return min(delay, MAX_RETRY_AFTER)


def http_error(status: int, text: str, retry_after=None) -> JudgeError:
    """Map an HTTP failure to a JudgeError. Bodies are withheld except the first 300 chars of a 422."""
    low = text.lower()
    if 400 <= status < 500 and ("max_tokens" in low or ("token" in low and "exceed" in low)):
        return JudgeError("JEV.CONTEXT_EXCEEDED", f"Jev HTTP {status}: request exceeds the model's context window", recoverable=True,
                          action="Shorten the evidence for this item.", status=status)
    if status == 401:
        return JudgeError("AUTH.CREDENTIALS", "Jev HTTP 401; response body withheld", action="Check TYPESAFE_API_KEY.", status=status)
    if status == 422:
        return JudgeError("JEV.VALIDATION", f"Jev HTTP 422: {text[:300]}", action="Fix the named field; usually a malformed question override.", status=status)
    code = {429: "JEV.RATE_LIMITED", 529: "JEV.OVERLOADED"}.get(status, "JEV.HTTP")
    transient = status in (429, 529) or status >= 500
    return JudgeError(code, f"Jev HTTP {status}; response body withheld", recoverable=transient,
                      action="Retry with backoff" if transient else "Check the request body and PLAN_ANCHOR_JEV_URL.",
                      status=status, retry_after=retry_after)


def call_jev(body: dict, api_key: str, timeout: float) -> dict:
    """One POST to Jev. Raises JudgeError; never retries or shortens (judge_with_policy does both)."""
    data = encode_body(body)
    req = urllib.request.Request(
        JEV_URL, data=data, method="POST",
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json", "User-Agent": "plan-anchoring/jev_judge"},
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read(MAX_RESPONSE_BYTES + 1)
    except urllib.error.HTTPError as exc:
        text = exc.read(4096).decode("utf-8", "replace")
        retry_after = parse_retry_after(exc.headers.get("Retry-After") if exc.headers is not None else None)
        raise http_error(exc.code, text, retry_after)
    except urllib.error.URLError as exc:
        raise JudgeError("JEV.NETWORK", f"Could not reach Jev: {exc.reason}", recoverable=True, action="Check network access and retry.")
    except (OSError, http.client.HTTPException) as exc:
        raise JudgeError("JEV.NETWORK", f"Jev connection failed: {type(exc).__name__}: {exc}", recoverable=True, action="Check network access and retry.")
    if len(raw) > MAX_RESPONSE_BYTES:
        raise JudgeError("JEV.RESPONSE_INVALID", "Jev response exceeded the size limit", recoverable=True, action="Retry once; if it repeats, check PLAN_ANCHOR_JEV_URL.")
    try:
        parsed = json.loads(raw.decode("utf-8"))
    except (ValueError, UnicodeError):
        raise JudgeError("JEV.RESPONSE_INVALID", "Jev response was not JSON", recoverable=True, action="Retry once; if it repeats, check PLAN_ANCHOR_JEV_URL.")
    if not isinstance(parsed, dict):
        raise JudgeError("JEV.RESPONSE_INVALID", "Jev response was not a JSON object", recoverable=True, action="Check PLAN_ANCHOR_JEV_URL points at the systemone endpoint.")
    return parsed


def is_transient(exc: JudgeError) -> bool:
    if exc.code in TRANSIENT_CODES:
        return True
    return exc.code == "JEV.HTTP" and exc.status is not None and exc.status >= 500


def item_core(text: str) -> str:
    return text[: -len(TRUNCATION_MARKER)] if text.endswith(TRUNCATION_MARKER) else text


def halve_item(item: dict, min_item_chars: int, meta: dict) -> bool:
    """Keep the head of the item text and re-append the truncation marker. False when already at the floor."""
    core = item_core(item["text"])
    if len(core) <= min_item_chars:
        return False
    item["text"] = core[: max(len(core) // 2, min_item_chars)] + TRUNCATION_MARKER
    meta["item_shortened"] = True
    meta["item_chars_sent"] = len(item["text"])
    return True


def fit_request(body: dict, max_request_bytes: int, min_item_chars: int, meta: dict) -> None:
    """Pre-flight cap: shorten the item until the serialised request (state + questions) is under max_request_bytes."""
    item = body["state"]["item"]
    size = len(encode_body(body))
    while size > max_request_bytes:
        if not halve_item(item, min_item_chars, meta):
            exc = JudgeError("JEV.CONTEXT_EXCEEDED",
                             f"request is {size} bytes with the item at {len(item['text'])} chars, over --max-request-bytes {max_request_bytes}; "
                             f"the item is already at --min-item-chars {min_item_chars}",
                             action="Shorten the evidence for this item (names, counts and summaries only), shorten the question overrides, or raise --max-request-bytes.")
            exc.meta = meta
            raise exc
        size = len(encode_body(body))
    meta["request_bytes"] = size


def judge_with_policy(body: dict, api_key: str, timeout: float, max_request_bytes: int, min_item_chars: int, questions: dict) -> tuple:
    """call_jev with the attempt policy and context handling. Returns (validated response, meta); raises JudgeError carrying .meta.

    - Transport-level failures (429, 529, 5xx, network, invalid response): up to MAX_ATTEMPTS attempts; the first
      retry waits Retry-After when given, else RETRY_DELAYS[0]; the next waits RETRY_DELAYS[1].
    - Context overflow (4xx mentioning max_tokens / token ... exceed): halve the item and retry until it fits or
      the item is at min_item_chars, then give up with JEV.CONTEXT_EXCEEDED.
    - Everything else (401, 422) is raised at once.
    """
    item = body["state"]["item"]
    meta = {"attempts": 0, "item_chars_sent": len(item["text"]), "item_shortened": False}
    fit_request(body, max_request_bytes, min_item_chars, meta)
    transient_failures = 0
    while True:
        meta["attempts"] += 1
        meta["item_chars_sent"] = len(item["text"])
        try:
            resp = call_jev(body, api_key, timeout)
            return validate_response(resp, questions), meta
        except JudgeError as exc:
            exc.meta = meta
            if exc.code == "JEV.CONTEXT_EXCEEDED":
                if halve_item(item, min_item_chars, meta):
                    meta["request_bytes"] = len(encode_body(body))
                    continue
                final = JudgeError("JEV.CONTEXT_EXCEEDED",
                                   f"Jev rejected the request for context size with the item at {len(item['text'])} chars "
                                   f"(floor --min-item-chars {min_item_chars}): {exc}",
                                   action="Shorten the evidence for this item: sprint report and PR body summaries, file names and counts only; or lower --min-item-chars.",
                                   status=exc.status)
                final.meta = meta
                raise final
            if is_transient(exc) and transient_failures < MAX_ATTEMPTS - 1:
                delay = RETRY_DELAYS[min(transient_failures, len(RETRY_DELAYS) - 1)]
                if transient_failures == 0 and exc.retry_after is not None:
                    delay = exc.retry_after
                transient_failures += 1
                _sleep(delay)
                continue
            raise


def is_probability(value) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and 0 <= value <= 1


def noul_probability(ans: dict):
    for key in ("noul", "probability", "p"):
        if key in ans:
            return ans[key]
    return None


def answer_problem(ans, question) -> str:
    """Why an answer cannot be gated, or None when it can (modelled on process-porn's validator, minus the model check)."""
    if not isinstance(ans, dict):
        return "answer is not an object"
    kind = ans.get("type", "choice")
    if kind == "noul":
        if not is_probability(noul_probability(ans)):
            return "noul probability is missing or not in [0, 1]"
        return None
    if kind != "choice":
        return f"answer type {kind!r} is not choice or noul"
    criteria = question.get("criteria") if isinstance(question, dict) else None
    options = set(criteria) if isinstance(criteria, dict) else set()
    if "choice" not in ans:
        return "choice answer has no 'choice'"
    if options and ans["choice"] not in options:
        return f"choice {ans['choice']!r} is not one of {sorted(options)}"
    if not is_probability(ans.get("confidence")):
        return "confidence is missing or not in [0, 1]"
    probs = ans.get("probabilities")
    if not isinstance(probs, dict):
        return "probabilities missing"
    if options and set(probs) != options:
        return "probability keys differ from the options"
    if not all(is_probability(p) for p in probs.values()):
        return "a probability is out of range"
    total = sum(probs.values())
    if abs(total - 1) > PROBABILITY_ROUNDING * max(len(probs), 1):
        return f"probabilities sum to {total:.4f}"
    return None


def validate_response(resp: dict, questions: dict) -> dict:
    if not isinstance(resp, dict) or not isinstance(resp.get("answers"), dict):
        raise JudgeError("JEV.RESPONSE_INVALID", "Jev response has no 'answers' object", recoverable=True,
                         action="Retry; if it repeats, check PLAN_ANCHOR_JEV_URL and the model name.")
    answers = resp["answers"]
    if set(answers) != set(questions):
        missing, extra = sorted(set(questions) - set(answers)), sorted(set(answers) - set(questions))
        raise JudgeError("JEV.RESPONSE_INVALID", f"Jev answer ids differ from the questions (missing {missing}, unexpected {extra})", recoverable=True,
                         action="Retry; if it repeats, check the question ids in the overrides.")
    for qid, q in questions.items():
        problem = answer_problem(answers[qid], q)
        if problem:
            raise JudgeError("JEV.RESPONSE_INVALID", f"invalid answer for {qid}: {problem}", recoverable=True,
                             action="Retry; if it repeats, the question set or the model changed shape.")
    return resp


def answer_choice(ans: dict) -> tuple:
    """Return (choice, confidence) for a choice answer; tolerate noul answers."""
    if ans.get("type") == "noul" or "noul" in ans:
        p = float(noul_probability(ans) or 0.0)
        return ("yes" if p >= 0.5 else "no", abs(p - 0.5) * 2)
    return (ans.get("choice", "unclear"), float(ans.get("confidence", 0.0)))


def decide(answers: dict, roles: dict, threshold: float) -> tuple:
    """Turn independent Jev answers into one verdict, gating in code (Jev cannot see its own answers)."""
    def get(role):
        qid = roles.get(role)
        if not qid or qid not in answers:
            return (None, 0.0)
        return answer_choice(answers[qid])

    rel, rel_c = get("relevance")
    comp, comp_c = get("compliance")
    conf, conf_c = get("conflict")
    evid, evid_c = get("evidence")
    min_conf = min(rel_c, comp_c, conf_c)

    if rel == "no" and rel_c >= threshold:
        return "not_applicable", min_conf
    if conf == "yes" and conf_c >= threshold:
        return "contradicted", min_conf
    if rel == "unclear" or rel_c < threshold or comp == "unclear" or comp_c < threshold:
        return "underspecified", min_conf
    if comp == "no":
        return "not_addressed", min_conf
    if comp == "yes":
        if roles.get("evidence") and evid == "no" and evid_c >= threshold:
            return "asserted_only", min_conf
        return "satisfied", min_conf
    return "underspecified", min_conf


def explain(verdict: str, min_conf: float, gate: str) -> str:
    noun = "bead" if gate == "plan" else "work"
    return {
        "contradicted": f"This {noun} proposes something the anchor rules out." if gate == "plan" else "The delivered work went against this anchor.",
        "not_addressed": f"This {noun} does not carry the directive forward." if gate == "plan" else "The delivered work does not show the directive was followed.",
        "asserted_only": "Compliance is asserted but no concrete evidence (paths, diffs, commands, tests) is cited.",
        "underspecified": f"Jev was not confident (min confidence {min_conf:.2f}); the anchor or the {noun} may be underspecified.",
        "satisfied": "The directive is carried forward." if gate == "plan" else "The directive was followed and evidenced.",
        "not_applicable": f"The anchor does not govern this {noun}.",
        "error": "Judging failed; see error.",
    }[verdict]


def read_item(args) -> dict:
    if args.item_file:
        try:
            text = pathlib.Path(args.item_file).read_text(encoding="utf-8")
        except FileNotFoundError:
            raise JudgeError("INPUT.MISSING", f"item file not found: {args.item_file}", action="Write the bead or work description to a file and pass it with --item-file.")
        except (OSError, UnicodeDecodeError) as exc:
            raise JudgeError("INPUT.MALFORMED", f"item file could not be read: {exc}", action="Pass a readable UTF-8 text file with --item-file.")
    else:
        text = sys.stdin.read()
    if not text.strip():
        raise JudgeError("INPUT.EMPTY", "item text is empty", action="Pass the bead or work description via --item-file.")
    truncated = False
    if len(text) > args.max_item_chars:
        text = text[: args.max_item_chars] + TRUNCATION_MARKER
        truncated = True
    return {"id": args.item_id, "title": args.item_title or "", "text": text, "truncated": truncated,
            "source": args.item_file or "stdin"}


def check_api_key(api_key: str) -> None:
    if not api_key:
        raise JudgeError("AUTH.CREDENTIALS", "TYPESAFE_API_KEY is not set", action="Export TYPESAFE_API_KEY, or use --dry-run / --answers-file.")
    if not api_key.isascii() or any(ord(c) <= 32 or ord(c) == 127 for c in api_key):
        raise JudgeError("AUTH.CREDENTIALS", "TYPESAFE_API_KEY has invalid formatting (whitespace, control or non-ASCII characters)",
                         action="Re-export TYPESAFE_API_KEY with the key exactly as issued.")


def judge_context(args, rec: dict) -> dict:
    """Shared pre-flight for judge and check-fixtures: options, question set, overrides, fixture answers, key and opt-in."""
    threshold = getattr(args, "threshold", DEFAULT_THRESHOLD)
    if not (0.0 <= threshold <= 1.0):
        raise JudgeError("VALIDATION.INPUT", f"--threshold must be between 0 and 1, got {threshold}")
    ctx = {
        "gate": args.gate, "model": getattr(args, "model", DEFAULT_MODEL), "threshold": threshold,
        "timeout": getattr(args, "timeout", 30.0),
        "max_item_chars": getattr(args, "max_item_chars", DEFAULT_MAX_ITEM_CHARS),
        "max_request_bytes": getattr(args, "max_request_bytes", DEFAULT_MAX_REQUEST_BYTES),
        "min_item_chars": getattr(args, "min_item_chars", DEFAULT_MIN_ITEM_CHARS),
        "dry_run": bool(getattr(args, "dry_run", False)),
    }
    for name in ("max_item_chars", "max_request_bytes", "min_item_chars"):
        if ctx[name] < 1:
            raise JudgeError("VALIDATION.INPUT", f"--{name.replace('_', '-')} must be a positive integer, got {ctx[name]}")
    ctx["base_qs"] = load_question_set(args.gate, getattr(args, "questions", ""))
    answers_file = getattr(args, "answers_file", "")
    ctx["answers_fixture"] = load_json(answers_file, "answers fixture") if answers_file else None
    if ctx["answers_fixture"] is not None and not isinstance(ctx["answers_fixture"], dict):
        raise JudgeError("INPUT.MALFORMED", "answers fixture must be an object keyed by anchor id (or '*')")
    overrides_file = getattr(args, "overrides_file", "")
    ctx["bead_overrides"] = load_json(overrides_file, "overrides file") if overrides_file else {}
    if not isinstance(ctx["bead_overrides"], dict):
        raise JudgeError("INPUT.MALFORMED", "overrides file must be an object keyed by anchor id")
    ctx["api_key"] = os.environ.get("TYPESAFE_API_KEY", "").strip()
    allow_remote = os.environ.get("PLAN_ANCHOR_ALLOW_REMOTE_JUDGE") == "1" or bool(getattr(args, "allow_remote", False))
    ctx["live"] = not ctx["dry_run"] and ctx["answers_fixture"] is None
    if ctx["live"]:
        check_api_key(ctx["api_key"])
        if not allow_remote:
            raise JudgeError("POLICY.REMOTE_JUDGE_DISABLED",
                             "Sending plan text to TypeSafe is disabled. The anchor statement and the item text would leave this machine.",
                             action="Set PLAN_ANCHOR_ALLOW_REMOTE_JUDGE=1 (or pass --allow-remote) once the data-retention question is settled for this plan; use --dry-run to inspect the request instead.")
    return ctx


def prepare_question_sets(ctx: dict, anchors: list) -> list:
    """Resolve every anchor's question set before any call, so a bad override aborts before anything leaves the machine."""
    prepared = []
    for a in anchors:
        record_override = (a.get("question_overrides") or {}).get(ctx["gate"]) or {}
        bead_override = ctx["bead_overrides"].get(a["id"]) or {}
        qs = merge_overrides(merge_overrides(ctx["base_qs"], record_override), bead_override)
        check_roles(qs, f"question set for {a['id']} after overrides")
        overridden = any(o.get(k) for o in (record_override, bead_override) for k in ("questions", "roles"))
        prepared.append((a, qs, bool(overridden)))
    return prepared


def judge_entries(ctx: dict, prepared: list, item: dict, fixture_key) -> tuple:
    """One Jev request per anchor. Returns (results, requests_out, model_answered)."""
    results, requests_out, model_answered = [], [], None
    for a, qs, overridden in prepared:
        body = {"model": ctx["model"], "state": build_state(a, item), "questions": qs["questions"]}
        entry = {
            "anchor_id": a["id"], "source_type": a["source_type"], "source_ref": a.get("source_ref"),
            "statement": a["statement"], "stated_by": a.get("stated_by"), "stated_at": a.get("stated_at"),
            "context": a.get("context"), "anchor_version": a.get("version", 1),
            "question_set": {"gate": qs.get("gate", ctx["gate"]), "version": qs.get("version"), "overridden": overridden},
            "questions_sha256": questions_digest(qs["questions"]),
            "deterministic_check": a.get("deterministic_check") or "",
        }
        if overridden:
            entry["questions"] = qs["questions"]
        meta = {"attempts": 0, "item_chars_sent": len(item["text"]), "item_shortened": False}
        if ctx["dry_run"]:
            try:
                fit_request(body, ctx["max_request_bytes"], ctx["min_item_chars"], meta)
                entry.update({"verdict": None, "answers": None, "min_confidence": None, "explanation": "dry run"})
            except JudgeError as exc:
                entry.update({"verdict": None, "answers": None, "min_confidence": None, "explanation": "dry run; request does not fit",
                              "error": exc.to_json()["error"]})
            requests_out.append(body)
            entry.update(meta)
            results.append(entry)
            continue
        try:
            if ctx["answers_fixture"] is not None:
                fx = ctx["answers_fixture"].get(fixture_key(a)) or ctx["answers_fixture"].get("*")
                if fx is None:
                    raise JudgeError("FIXTURE.MISSING", f"answers fixture has no entry for {fixture_key(a)} (or '*')")
                meta["attempts"] = 1
                resp = validate_response({"model": "fixture", "answers": fx}, qs["questions"])
            else:
                resp, meta = judge_with_policy(body, ctx["api_key"], ctx["timeout"], ctx["max_request_bytes"], ctx["min_item_chars"], qs["questions"])
            model_answered = resp.get("model", model_answered)
            verdict, min_conf = decide(resp["answers"], qs["roles"], ctx["threshold"])
            entry.update({"verdict": verdict, "answers": resp["answers"], "min_confidence": round(min_conf, 3),
                          "explanation": explain(verdict, min_conf, ctx["gate"]), "usage": resp.get("usage")})
        except JudgeError as exc:
            if exc.code in ABORT_CODES:
                raise
            meta = exc.meta or meta
            entry.update({"verdict": "error", "answers": None, "min_confidence": None, "explanation": explain("error", 0.0, ctx["gate"]),
                          "error": exc.to_json()["error"]})
        entry.update(meta)
        results.append(entry)
    results.sort(key=lambda r: (SEVERITY.get(r["verdict"] or "error", 99), r["anchor_id"]))
    return results, requests_out, model_answered


def assemble_run(ctx: dict, rec: dict, anchors_path: str, item: dict, results: list, requests_out: list, model_answered) -> dict:
    counts = {v: 0 for v in VERDICTS}
    for r in results:
        if r["verdict"]:
            counts[r["verdict"]] += 1
    run = {
        "schema": "plan-anchoring/run/1",
        "run_id": f"{ctx['gate']}-{stamp()}-{sanitize_id(item['id'])}",
        "gate": ctx["gate"],
        "created_at": now_utc().isoformat(timespec="seconds"),
        "anchors_path": anchors_path,
        "plan": rec.get("plan"),
        "item": {"id": item["id"], "title": item["title"], "source": item["source"], "chars": len(item["text"]), "truncated": item["truncated"]},
        "model_requested": ctx["model"],
        "model_answered": model_answered,
        "threshold": ctx["threshold"],
        "max_request_bytes": ctx["max_request_bytes"],
        "min_item_chars": ctx["min_item_chars"],
        "dry_run": ctx["dry_run"],
        "fixture": ctx["answers_fixture"] is not None,
        "results": results,
        "summary": {
            "counts": {k: v for k, v in counts.items() if v},
            "failing": [r["anchor_id"] for r in results if r["verdict"] in FAILING],
            "errors": [r["anchor_id"] for r in results if r["verdict"] == "error"],
            "shortened": [r["anchor_id"] for r in results if r.get("item_shortened")],
            "pass": not any(r["verdict"] in FAILING or r["verdict"] == "error" for r in results) and not ctx["dry_run"],
        },
    }
    if ctx["dry_run"]:
        run["requests"] = requests_out
        run["totals"] = {"requests": len(requests_out), "bytes": sum(len(encode_body(b)) for b in requests_out),
                         "max_request_bytes": ctx["max_request_bytes"]}
    return run


def write_run(run: dict, out_path: str, run_dir: pathlib.Path) -> str:
    """Write the run file; without --out, two runs of one item in the same second get distinct names."""
    if not out_path:
        base_id, n = run["run_id"], 1
        while (run_dir / f"{run['run_id']}.json").exists():
            n += 1
            run["run_id"] = f"{base_id}-{n}"
        out_path = str(run_dir / f"{run['run_id']}.json")
    save_json(out_path, run)
    run["path"] = out_path
    run["run_path"] = out_path
    return out_path


def default_run_dir(args) -> pathlib.Path:
    return pathlib.Path(args.run_dir or str(pathlib.Path(args.anchors).parent / "anchor-runs"))


def cmd_judge(args) -> dict:
    rec = load_record(args.anchors)
    require_valid(rec)
    if args.all:
        anchors = [a for a in rec["anchors"] if a["status"] in JUDGED_STATUSES]
    else:
        if not args.anchor:
            raise JudgeError("VALIDATION.INPUT", "pass --anchor ID (repeatable) or --all")
        anchors = [find_anchor(rec, aid) for aid in dict.fromkeys(args.anchor)]
        if not args.include_unlocked:
            unlocked = [a["id"] for a in anchors if a.get("status") not in JUDGED_STATUSES]
            if unlocked:
                raise JudgeError("ANCHOR.NOT_LOCKED", f"anchors not locked or promoted: {', '.join(unlocked)}",
                                 action="Lock the anchor first, or pass --include-unlocked to judge a proposed or retired anchor deliberately.")
    if not anchors:
        raise JudgeError("ANCHOR.NONE", "no locked anchors to judge", action="Lock at least one anchor, or pass --anchor.")
    ctx = judge_context(args, rec)
    item = read_item(args)
    prepared = prepare_question_sets(ctx, anchors)
    results, requests_out, model_answered = judge_entries(ctx, prepared, item, lambda a: a["id"])
    run = assemble_run(ctx, rec, args.anchors, item, results, requests_out, model_answered)
    write_run(run, args.out, default_run_dir(args))
    return {"success": True, "data": run, "error": None}


def cmd_check_fixtures(args) -> dict:
    rec = load_record(args.anchors)
    require_valid(rec)
    if args.all:
        anchors = [a for a in rec["anchors"] if a.get("status") != "retired"]
    elif args.id:
        anchors = [find_anchor(rec, args.id)]
    else:
        raise JudgeError("VALIDATION.INPUT", "pass --id ANC-NNN or --all")
    ctx = judge_context(args, rec)
    prepared = prepare_question_sets(ctx, anchors)
    fixtures_dir = (pathlib.Path(args.run_dir) / "fixtures") if args.run_dir else None
    checks, runs, written = [], [], []
    for a, qs, overridden in prepared:
        fx = a.get("fixtures") or {}
        for kind in ("pass", "fail"):
            for n, sentence in enumerate(fx.get(kind) or [], 1):
                if not isinstance(sentence, str) or not sentence.strip():
                    continue
                item = {"id": f"fixture:{a['id']}:{kind}:{n}", "title": "fixture", "text": sentence.strip(), "truncated": False, "source": "fixtures"}
                key = f"{a['id']}:{kind}:{n}"
                results, requests_out, model_answered = judge_entries(ctx, [(a, qs, overridden)], item, lambda _a, key=key: key)
                run = assemble_run(ctx, rec, args.anchors, item, results, requests_out, model_answered)
                verdict = results[0]["verdict"]
                check = {"anchor_id": a["id"], "kind": kind, "index": n, "sentence": item["text"], "verdict": verdict,
                         "expected": list(FIXTURE_EXPECTED[kind]),
                         "expected_ok": (verdict in FIXTURE_EXPECTED[kind]) if not ctx["dry_run"] else None}
                if results[0].get("error"):
                    check["error"] = results[0]["error"]
                if fixtures_dir is not None:
                    written.append(write_run(run, "", fixtures_dir))
                    check["run_path"] = run["run_path"]
                checks.append(check)
                runs.append(run)
    if not checks:
        raise JudgeError("FIXTURE.NONE", "no fixture sentences recorded on the selected anchors",
                         action="Add fixtures with `update --id ANC-NNN --fixture-pass ... --fixture-fail ...`.")
    mismatches = [f"{c['anchor_id']}:{c['kind']}:{c['index']}" for c in checks if c["expected_ok"] is False]
    data = {"gate": args.gate, "dry_run": ctx["dry_run"], "fixture": ctx["answers_fixture"] is not None,
            "checks": checks, "mismatches": mismatches, "pass": not mismatches and not ctx["dry_run"],
            "path": str(fixtures_dir) if fixtures_dir is not None else None,
            "run_path": str(fixtures_dir) if fixtures_dir is not None else None, "runs": written}
    if ctx["dry_run"]:
        data["requests"] = [b for r in runs for b in r.get("requests", [])]
        data["totals"] = {"requests": len(data["requests"]), "bytes": sum(len(encode_body(b)) for b in data["requests"]),
                          "max_request_bytes": ctx["max_request_bytes"]}
    return {"success": True, "data": data, "error": None}


# --------------------------------------------------------------------------- #
# reporting
# --------------------------------------------------------------------------- #
def quote_statement(statement: str) -> str:
    s = statement.strip()
    return f'"{s}"' if s.endswith((".", "!", "?")) else f'"{s}."'


def report_lines(run: dict, include_passing: bool = False) -> list:
    lines = []
    item_id = run["item"]["id"]
    for r in run["results"]:
        v = r.get("verdict") or "error"
        if v in ("satisfied", "not_applicable") and not include_passing:
            continue
        label = v.replace("_", " ")
        if r.get("source_type") == "ANC":
            who = f"You said ({r.get('context') or 'planning session'}, {short_date(r.get('stated_at') or '')})"
        else:
            register = REGISTER_NAMES.get(r.get("source_type"), str(r.get("source_type")))
            who = f"{r.get('source_ref') or r['anchor_id']} ({register}) says"
        lines.append(f"> **{item_id} · {r['anchor_id']} · {label}** {who}: {quote_statement(r['statement'])} {r.get('explanation', '')}".rstrip())
        if v == "error" and isinstance(r.get("error"), dict) and r["error"].get("code"):
            lines.append(f">   Error {r['error']['code']}: {r['error'].get('message', '')}".rstrip())
        if r.get("deterministic_check") and v in ("satisfied", "asserted_only") and run["gate"] == "dev":
            lines.append(f">   Pair with the deterministic check: {r['deterministic_check']}")
        lines.append("")
    return lines


def cmd_report(args) -> dict:
    run = load_run(args.run, "run file")
    s = run["summary"]
    head = [f"## Anchor gate: {run['gate']} · {run['item']['id']}" + (f" - {run['item']['title']}" if run['item'].get('title') else ""), "",
            f"Run `{run['run_id']}`, model `{run.get('model_answered') or run.get('model_requested')}`, threshold {run['threshold']}. "
            + ("DRY RUN. " if run.get("dry_run") else "") + ("FIXTURE answers. " if run.get("fixture") else "")
            + "Counts: " + (", ".join(f"{k} {v}" for k, v in (s.get("counts") or {}).items()) or "none") + ".", ""]
    body = report_lines(run, include_passing=args.all)
    if not body:
        body = ["All judged anchors are satisfied or not applicable.", ""]
    md = "\n".join(head + body).rstrip() + "\n"
    if args.format == "md":
        return md
    return {"success": True, "data": {"run_id": run["run_id"], "pass": s.get("pass"), "failing": s.get("failing"), "markdown": md}, "error": None}


def collect_runs(paths: list) -> list:
    runs = []
    for p in paths:
        pp = pathlib.Path(p)
        files = sorted(pp.glob("*.json")) if pp.is_dir() else [pp]
        for f in files:
            try:
                r = json.loads(f.read_text(encoding="utf-8"))
            except (json.JSONDecodeError, OSError, UnicodeDecodeError):
                continue
            if isinstance(r, dict) and r.get("schema") == "plan-anchoring/run/1" and not r.get("dry_run") \
                    and isinstance(r.get("results"), list) and isinstance(r.get("item"), dict):
                runs.append(r)
    return runs


def cmd_coverage(args) -> dict:
    runs = collect_runs(args.runs)
    if args.gate:
        runs = [r for r in runs if r.get("gate") == args.gate]
    if not runs:
        raise JudgeError("RUNS.NONE", "no run files found", action="Run `judge` first, or check the --runs path.")
    # latest run per (gate, item) wins
    latest = {}
    for r in sorted(runs, key=lambda r: str(r.get("created_at", ""))):
        latest[(r["gate"], r["item"]["id"])] = r

    # without a --gate filter, columns are qualified so plan and dev verdicts for one item never overwrite each other
    def column(gate, item_id):
        return item_id if args.gate else f"{gate}:{item_id}"
    matrix = {}
    items = sorted({column(g, i) for g, i in latest})
    for (gate, item_id), r in latest.items():
        for res in r["results"]:
            matrix.setdefault(res["anchor_id"], {})[column(gate, item_id)] = res["verdict"]
    anchors_sorted = sorted(matrix)
    uncovered = [a for a in anchors_sorted if not any(v == "satisfied" for v in matrix[a].values())]
    contradicted = {a: [i for i, v in matrix[a].items() if v == "contradicted"] for a in anchors_sorted if any(v == "contradicted" for v in matrix[a].values())}
    data = {"gate": args.gate or "all", "items": items, "anchors": anchors_sorted, "matrix": matrix,
            "uncovered_anchors": uncovered, "contradicted": contradicted, "runs_considered": len(latest)}
    if args.format == "md":
        lines = [f"## Anchor coverage ({data['gate']})", "", "| Anchor | " + " | ".join(items) + " | Covered |", "| --- | " + " | ".join("---" for _ in items) + " | --- |"]
        for a in anchors_sorted:
            row = [(matrix[a].get(i) or "-").replace("_", " ") for i in items]
            lines.append(f"| {a} | " + " | ".join(row) + f" | {'no' if a in uncovered else 'yes'} |")
        lines.append("")
        if uncovered:
            lines.append("Uncovered anchors (no item satisfies them): " + ", ".join(uncovered))
        return "\n".join(lines) + "\n"
    return {"success": True, "data": data, "error": None}


def diff_rank(verdict) -> int:
    """Severity index for diffs; an error (or a missing verdict) ranks below every real verdict, so becoming one is a regression."""
    if verdict is None or verdict == "error":
        return -1
    return SEVERITY.get(verdict, -1)


def cmd_diff(args) -> dict:
    prev = load_run(args.prev, "previous run")
    nxt = load_run(args.next, "next run")
    if prev["gate"] != nxt["gate"]:
        raise JudgeError("RUNS.MISMATCH", "runs are for different gates")
    if prev["item"]["id"] != nxt["item"]["id"]:
        raise JudgeError("RUNS.MISMATCH", f"runs are for different items ({prev['item']['id']} vs {nxt['item']['id']})",
                         action="Diff two runs of the same bead or sprint.")
    pv = {r["anchor_id"]: r for r in prev["results"]}
    nv = {r["anchor_id"]: r for r in nxt["results"]}
    changed, added, removed = [], [], []
    for aid in sorted(set(pv) | set(nv)):
        if aid not in pv:
            added.append({"anchor_id": aid, "verdict": nv[aid]["verdict"]})
        elif aid not in nv:
            removed.append({"anchor_id": aid, "verdict": pv[aid]["verdict"]})
        elif pv[aid]["verdict"] != nv[aid]["verdict"]:
            changed.append({"anchor_id": aid, "from": pv[aid]["verdict"], "to": nv[aid]["verdict"],
                            "improved": diff_rank(nv[aid]["verdict"]) > diff_rank(pv[aid]["verdict"])})
    data = {"gate": nxt["gate"], "item": nxt["item"]["id"], "prev": prev["run_id"], "next": nxt["run_id"],
            "changed": changed, "added": added, "removed": removed,
            "regressions": [c["anchor_id"] for c in changed if not c["improved"]],
            "pass_before": prev["summary"].get("pass"), "pass_after": nxt["summary"].get("pass")}
    return {"success": True, "data": data, "error": None}


# --------------------------------------------------------------------------- #
# cli
# --------------------------------------------------------------------------- #
def add_judge_options(parser, with_item: bool) -> None:
    parser.add_argument("--anchors", required=True)
    parser.add_argument("--gate", choices=("plan", "dev"), required=True)
    if with_item:
        parser.add_argument("--anchor", action="append", help="anchor id; repeatable")
        parser.add_argument("--all", action="store_true", help="all locked/promoted anchors")
        parser.add_argument("--include-unlocked", action="store_true", help="allow --anchor to name proposed or retired anchors")
        parser.add_argument("--item-id", required=True, help="bead id, PR number, or sprint id")
        parser.add_argument("--item-title", default="")
        parser.add_argument("--item-file", default="", help="file with the bead or work description; stdin if omitted")
        parser.add_argument("--max-item-chars", type=int, default=DEFAULT_MAX_ITEM_CHARS)
        parser.add_argument("--out", default="", help="explicit run file path")
    else:
        parser.add_argument("--id", default="", help="one anchor id")
        parser.add_argument("--all", action="store_true", help="every non-retired anchor that has fixtures")
    parser.add_argument("--questions", default="", help="question-set JSON; default assets/questions/<gate>.json")
    parser.add_argument("--overrides-file", default="", help="JSON {anchor_id: {questions: {...}, roles: {...}}} read from the question beads; merged after anchors.json overrides")
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--threshold", type=float, default=DEFAULT_THRESHOLD)
    parser.add_argument("--max-request-bytes", type=int, default=DEFAULT_MAX_REQUEST_BYTES,
                        help="shorten the item until each serialised request is under this many bytes")
    parser.add_argument("--min-item-chars", type=int, default=DEFAULT_MIN_ITEM_CHARS,
                        help="never shorten the item below this on a context overflow; record JEV.CONTEXT_EXCEEDED instead")
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--run-dir", default="", help="judge: default <anchors dir>/anchor-runs; check-fixtures: write runs under <run-dir>/fixtures/")
    parser.add_argument("--dry-run", action="store_true", help="write the Jev requests; call nothing")
    parser.add_argument("--answers-file", default="", help="fixture answers keyed by anchor id (check-fixtures: '<anchor>:pass:<n>') or '*'; no network")
    parser.add_argument("--allow-remote", action="store_true", help="same as PLAN_ANCHOR_ALLOW_REMOTE_JUDGE=1")


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="jev_judge.py", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)

    a = sub.add_parser("add", help="append an anchor record")
    a.add_argument("--anchors", required=True)
    a.add_argument("--statement", required=True, help="the user's exact words")
    a.add_argument("--stated-by", default=os.environ.get("USER", "user"))
    a.add_argument("--stated-at", default="", help="ISO date; default today's local date")
    a.add_argument("--context", default="planning session")
    a.add_argument("--summary", default="", help="short paraphrase for tables only; never judged")
    a.add_argument("--scope", default="", help="where the anchor applies; feeds the relevance question")
    a.add_argument("--deterministic-check", default="", help="a code-level check to pair with the dev gate")
    a.add_argument("--source-type", choices=SOURCE_TYPES, default="ANC")
    a.add_argument("--source-ref", default="", help="REQ-014 / ADR-0007 when source type is REQ/ADR")
    a.add_argument("--status", choices=STATUSES, default="proposed", help="proposed or locked; retired is refused")
    a.add_argument("--plan", default="phase-current", help="used only when creating a new anchors.json")
    a.add_argument("--intent-ref", default="doc/plan/phase-current/intent.md")
    a.set_defaults(func=cmd_add)

    u = sub.add_parser("update", help="edit scope, summary, deterministic check or fixtures (never the statement)")
    u.add_argument("--anchors", required=True)
    u.add_argument("--id", required=True)
    u.add_argument("--scope")
    u.add_argument("--summary")
    u.add_argument("--deterministic-check")
    u.add_argument("--fixture-pass", action="append", help="a sentence a bead would say if it satisfied the anchor; repeatable")
    u.add_argument("--fixture-fail", action="append", help="a sentence a bead would say if it violated the anchor; repeatable")
    u.add_argument("--clear-fixtures", action="store_true", help="drop existing fixtures before adding the given ones")
    u.add_argument("--statement", help=argparse.SUPPRESS)
    u.set_defaults(func=cmd_update)

    rt = sub.add_parser("retire", help="retire an anchor (status retired, version bump, history event)")
    rt.add_argument("--anchors", required=True)
    rt.add_argument("--id", required=True)
    rt.add_argument("--replaced-by", default="", help="the anchor that carries the changed directive")
    rt.add_argument("--reason", default="")
    rt.set_defaults(func=cmd_retire)

    v = sub.add_parser("validate", help="check anchors.json")
    v.add_argument("--anchors", required=True)
    v.set_defaults(func=cmd_validate)

    r = sub.add_parser("render", help="write anchors.md")
    r.add_argument("--anchors", required=True)
    r.add_argument("--out", default="")
    r.set_defaults(func=cmd_render)

    sb = sub.add_parser("set-beads", help="record bead ids on an anchor")
    sb.add_argument("--anchors", required=True)
    sb.add_argument("--id", required=True)
    sb.add_argument("--anchor-bead", default="")
    sb.add_argument("--plan-bead", default="")
    sb.add_argument("--dev-bead", default="")
    sb.set_defaults(func=cmd_set_beads)

    pr = sub.add_parser("promote", help="promote an anchor to an ADR")
    pr.add_argument("--anchors", required=True)
    pr.add_argument("--id", required=True)
    pr.add_argument("--adr", required=True)
    pr.set_defaults(func=cmd_promote)

    ci = sub.add_parser("compose-item", help="build the labelled item text from bd JSON")
    ci.add_argument("--show-json", required=True, help="output of `bd show <id> --json` (file, or - for stdin)")
    ci.add_argument("--children-json", default="", help="output of `bd children <id> --json`; closed children are appended")
    ci.add_argument("--out", default="", help="write the text here; without it the text is returned in data.text")
    ci.set_defaults(func=cmd_compose_item)

    j = sub.add_parser("judge", help="judge one item against anchors")
    add_judge_options(j, with_item=True)
    j.set_defaults(func=cmd_judge)

    cf = sub.add_parser("check-fixtures", help="judge each anchor's fixture sentences; pass -> satisfied, fail -> not_addressed or contradicted")
    add_judge_options(cf, with_item=False)
    cf.set_defaults(func=cmd_check_fixtures)

    rp = sub.add_parser("report", help="render a run")
    rp.add_argument("--run", required=True)
    rp.add_argument("--all", action="store_true", help="include satisfied and not-applicable rows")
    rp.add_argument("--format", choices=("json", "md"), default="json")
    rp.set_defaults(func=cmd_report)

    cv = sub.add_parser("coverage", help="anchors x items matrix")
    cv.add_argument("--runs", nargs="+", required=True, help="run files or directories")
    cv.add_argument("--gate", choices=("plan", "dev"), default="")
    cv.add_argument("--format", choices=("json", "md"), default="json")
    cv.set_defaults(func=cmd_coverage)

    d = sub.add_parser("diff", help="compare two runs")
    d.add_argument("--prev", required=True)
    d.add_argument("--next", required=True)
    d.set_defaults(func=cmd_diff)
    return p


def exit_code_for(cmd: str, out) -> int:
    if not isinstance(out, dict):
        return 0
    if out.get("success") is False:
        return 1
    data = out.get("data") or {}
    if cmd == "judge" and not data.get("dry_run") and not (data.get("summary") or {}).get("pass"):
        return 1
    if cmd == "check-fixtures" and data.get("mismatches"):
        return 1
    return 0


def main(argv=None) -> int:
    args = build_parser().parse_args(argv)
    fmt = getattr(args, "format", "json")
    try:
        out = args.func(args)
    except JudgeError as exc:
        emit(exc.to_json())
        return 2
    except KeyboardInterrupt:
        emit({"success": False, "canceled": True, "aborted_by": "user", "data": None, "error": None})
        return 130
    except Exception as exc:  # noqa: BLE001 - a JSON error object beats a traceback for the calling agent
        emit(JudgeError("SCRIPT.INTERNAL", f"{type(exc).__name__}: {exc}",
                        action="Report this with the command line; check the JSON inputs against references/anchor-document.md.").to_json())
        return 2
    emit(out, fmt)
    return exit_code_for(args.cmd, out)


if __name__ == "__main__":
    sys.exit(main())
