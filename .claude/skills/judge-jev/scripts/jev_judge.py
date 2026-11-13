#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "pydantic>=2.7,<3",
#   "pyyaml>=5.1",
#   "pyoxigraph>=0.5,<0.6",
#   "pyshacl>=0.30,<1",
#   "sc-observability==1.5.0",
# ]
# ///
"""jev_judge.py

Run it with ``uv run --script jev_judge.py ...``: the inline metadata above
supplies its five dependencies. pydantic (v2) defines the model of every JSON
input (ADR-JEV-0005) and is imported at the top (DEPENDENCY.PYDANTIC when it
does not import); the others are imported lazily where they are needed (pyoxigraph and pyshacl only when a definitions file is read). The
run log goes through the sc-observability logger (``sc-observability==1.5.0``,
Python 3.11+; DEPENDENCY.SC_OBSERVABILITY when it does not import), and a live
Jev call reads the committed ``allow_remote_judge`` answer through
``judge_jev_settings.py`` (PyYAML when the settings file exists). Everything
else is the standard library.

Every JSON input (quality tree, question set, answers file, Raptor import index,
batch request line, Jev's answers) is parsed by ``strict_loads`` (NaN and
Infinity, oversized integers, deep nesting and lone surrogates are refused there)
and then by one strict pydantic model; a refusal names the field path and the
rule it breaks. ``validate`` runs those models and nothing else.

Expected failures travel as values: a function that can fail returns
``Ok(value)`` or ``Err(JudgeError)`` (see ``result.py``) and its caller
propagates the ``Err`` explicitly. ``main`` is the only place an error becomes
JSON on stdout and an exit code; anything unexpected that still raises is
reported there as SCRIPT.INTERNAL.

Subcommands
  batch           low layer: JSONL request lines (stdin or --input) -> one JSONL result line per (item, candidate,
                  repeat) on stdout as each answer arrives; each request, answer and result is also a log event in the
                  sc-observability log <run-dir>/logs/judge-jev.log.jsonl; never writes beads
  quality         one or more REQs, ADRs or NFRs (repeated --req|--adr|--nfr <id>) from a Raptor import index
                  (--index), each judged with its kind's decision tree (assets/questions/quality-<kind>.json or
                  --questions): one batch request per wave, or per statement of a per-statement wave; every wave
                  reports each question's answer (per statement) as its reason;
                  answers pick the next wave, and the last wave's verdict_rules (or a leaf) give the verdict; every
                  wave is logged, all records under one run name; a record's definitions (--definitions, default the
                  .sc/judge-jev/ definitions files) ride in the request state (ADR-JEV-0008)
  compare-definition
                  runs the requests one or more candidate definitions reach, without them and with them, and
                  reports each answer's choice and confidence on both sides and whether the definitions are material:
                  an answer flips or its confidence crosses the decision threshold (REQ-JEV-0213)
  doctor          checks every tree, question set and definitions file of the repository scope, calls nothing
                  (REQ-JEV-0214, REQ-JEV-0215)

Every subcommand prints JSON to stdout (or markdown with --format md where
supported).

Exit codes
  A tool failure is an exit code; a negative answer from a command that ran
  properly is not. There is no exit 1.
  2  the command could not do its job: {"success": false, "data": null|{...},
     "error": {code, message, recoverable, suggested_action}} (bad input or
     arguments as VALIDATION.INPUT, a missing or unreadable file, a malformed
     record or override, bad credentials, remote judging not allowed by
     allow_remote_judge in .sc/judge-jev/settings.yaml as
     POLICY.REMOTE_JUDGE_DISABLED, a missing dependency, an internal error as
     SCRIPT.INTERNAL, and any Jev call that produced no answer: a Jev HTTP
     error, a network or transport failure, an invalid response, or a missing
     or malformed answers-file entry)
  130 interrupted: the error envelope with code CANCELED (recoverable true)
  `batch` stdout carries only result lines: exit 0 when every
  line has an answer (or --dry-run); exit 2 with the error envelope on
  stderr when any line has none (each such line has ok false and its error);
  an error before the first line prints the envelope on stdout as usual.
"""
from __future__ import annotations

import argparse
import asyncio
import collections
import concurrent.futures
import datetime as _dt
import hashlib
import http.client
import json
import math
import os
import pathlib
import re
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
from typing import Annotated, Any, Literal, NamedTuple, Optional, Union

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from result import Err, Ok, Result, utf8_stdio  # noqa: E402
from jev_errors import BEARER_RE, JudgeError, redact_sensitive, _redact_text  # noqa: E402,F401  (standard library only)
import jev_definitions  # noqa: E402  (standard library only)
import jev_runs  # noqa: E402  (standard library at import)

try:
    from pydantic import (AfterValidator, BaseModel, ConfigDict, Discriminator, Field, PrivateAttr, RootModel, Tag,
                          ValidationError, model_validator)
    from pydantic_core import PydanticCustomError
except ImportError as _exc:  # before any model exists: print the typed error by hand and stop
    if __name__ != "__main__":
        raise
    print(json.dumps({"success": False, "data": None, "error": {
        "code": "DEPENDENCY.PYDANTIC", "message": f"pydantic (v2) is required for the input models and did not import: {_exc}",
        "recoverable": False, "suggested_action": "Run with `uv run --script jev_judge.py ...` (its inline metadata pins "
                                                  "pydantic>=2.7,<3), or python3 -m pip install 'pydantic>=2.7,<3'."}}, indent=2))
    sys.exit(2)


JEV_URL = "https://api.typesafe.ai/v1/systemone"  # tests redirect by patching this constant in-process
TYPESAFE_KEY_ENV = "TYPESAFE_API_KEY"
CREDENTIAL_SERVICE = "plan-anchor"
CREDENTIAL_ACCOUNT = "typesafe-api-key"
WINDOWS_CREDENTIAL_TARGET = f"{CREDENTIAL_SERVICE}:{CREDENTIAL_ACCOUNT}"
DEFAULT_MODEL = "jev-latest"
DEFAULT_THRESHOLD = 0.6
DEFAULT_MAX_REQUEST_BYTES = 24000  # plan gate: what Jev reads reliably (a planted contradiction 28k deep scored 0.46 vs 0.79 near the top)
DEFAULT_MAX_REQUEST_BYTES_DEV = 64000  # dev gate: evidence is long; Jev accepted 107k bytes in one attempt, halving never fired in 245 requests
MAX_REQUEST_BYTES_CEILING = 64000  # every request is logged whole, and the sc-observability logger refuses events over 64 KiB
MAX_RESPONSE_BYTES = 1048576
RETRY_DELAYS = (1.0, 2.0)          # seconds before the second and third attempt; Retry-After overrides the first
MAX_RETRY_AFTER = 30.0
PROBABILITY_ROUNDING = 0.01        # Jev rounds each probability; the sum may be off by this per option
HERE = pathlib.Path(__file__).resolve().parent
QUESTIONS_DIR = HERE.parent / "assets" / "questions"
KINDS = ("directive", "prohibition", "structural")  # how decide() gates: see decide(); structural anchors skip Jev at the plan gate
REQUIRED_ROLES = ("relevance", "compliance", "conflict")
ABORT_CODES = ("AUTH.CREDENTIALS", "POLICY.REMOTE_JUDGE_DISABLED")
TRANSIENT_CODES = ("JEV.RATE_LIMITED", "JEV.OVERLOADED", "JEV.NETWORK", "JEV.RESPONSE_INVALID")

_sleep = time.sleep  # replaced in tests
BATCH_ID_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}")  # matched whole (fullmatch): a trailing newline is not part of an id
MAX_PROBLEMS = 5                    # pydantic errors named in one refusal message
MAX_MESSAGE = 300                   # characters of one problem in a refusal message
ATTEMPT_MESSAGE_LIMIT = 500         # characters of an error message recorded in a result line: bounds its log events
FIXTURE_CODE_LIMIT = 64             # characters of an answers-file {"__error__": code}: the code is logged unclipped, so it is bounded at load
WORST_CHAR = "\U0001F600"           # the character that measures largest in the logger's ASCII-escaped JSON (12 bytes: a surrogate pair)
WORST_PROBABILITY = 1.2345678901234567e-300  # the longest repr a probability in [0, 1] can have: sizes the largest possible answer


# --------------------------------------------------------------------------- #
# plumbing
# --------------------------------------------------------------------------- #
def fail(code: str, message: str, **fields) -> Err[JudgeError]:
    """An expected failure as a value: ``return fail(...)`` where the old code raised."""
    return Err(JudgeError(code, message, **fields))


EMIT_MAX_DEPTH = 100  # containers nested deeper than this are replaced by a marker in emitted output (json.dumps(indent=) recurses)
EMIT_TRUNCATED = "<truncated: nested deeper than %d levels>" % EMIT_MAX_DEPTH


def bound_depth(value, limit: int = EMIT_MAX_DEPTH):
    """A copy of ``value`` whose containers nest at most ``limit`` deep; a deeper container becomes EMIT_TRUNCATED. Iterative."""
    if not isinstance(value, (dict, list, tuple)):
        return value
    root = {} if isinstance(value, dict) else []
    stack = [(value, root, 1)]
    while stack:
        source, out, depth = stack.pop()
        for key, child in (source.items() if isinstance(source, dict) else enumerate(source)):
            if isinstance(child, (dict, list, tuple)):
                if depth >= limit:
                    kid = EMIT_TRUNCATED
                else:
                    kid = {} if isinstance(child, dict) else []
                    stack.append((child, kid, depth + 1))
            else:
                kid = child
            if isinstance(out, dict):
                out[key] = kid
            else:
                out.append(kid)
    return root


def _reject_constant(name: str):
    raise ValueError(f"{name} is not a JSON number")


JSON_MAX_DEPTH = 500  # deepest container nesting accepted in any JSON input, on every Python (3.14's parser has no recursion limit)


def _json_depth_exceeds(value, limit: int) -> bool:
    """True when ``value``'s containers nest deeper than ``limit``. Iterative, so any depth is measured without recursing."""
    stack = [(value, 1)] if isinstance(value, (dict, list)) else []
    while stack:
        node, depth = stack.pop()
        if depth > limit:
            return True
        stack.extend((child, depth + 1) for child in (node.values() if isinstance(node, dict) else node)
                     if isinstance(child, (dict, list)))
    return False


def strict_loads(text: str):
    """json.loads that also refuses what later breaks a request or the log: NaN and Infinity, a string that is not
    encodable UTF-8 (a lone surrogate escape), containers nested deeper than JSON_MAX_DEPTH. Raises ValueError (oversized
    integers, deep nesting and malformed JSON included) or RecursionError (nesting too deep for the parser); callers turn
    both into a typed error. The depth limit makes the refusal the same on every Python: 3.11-3.13 raise RecursionError
    near 1000 levels, 3.14 parses any depth."""
    value = json.loads(text, parse_constant=_reject_constant)
    if _json_depth_exceeds(value, JSON_MAX_DEPTH):
        raise ValueError(f"containers are nested deeper than {JSON_MAX_DEPTH} levels")
    json.dumps(value, ensure_ascii=False, allow_nan=False).encode("utf-8")
    return value


def utf8_text(text: str) -> str:
    """Text that encodes as UTF-8; JSON-escaped (ASCII) when it holds a lone surrogate, so output never fails."""
    try:
        text.encode("utf-8")
        return text
    except UnicodeEncodeError:
        return text.encode("utf-8", "backslashreplace").decode("utf-8")


def emit(obj, fmt: str = "json") -> None:
    obj = bound_depth(redact_sensitive(obj))  # always printable: nesting beyond EMIT_MAX_DEPTH is marked, never a traceback
    if fmt == "json":
        text = json.dumps(obj, indent=2, ensure_ascii=False)
        try:
            text.encode("utf-8")
        except UnicodeEncodeError:
            text = json.dumps(obj, indent=2, ensure_ascii=True)
        print(text)
    else:
        print(utf8_text(str(obj)))


def now_utc() -> _dt.datetime:
    return _dt.datetime.now(_dt.timezone.utc)


def stamp() -> str:
    return now_utc().strftime("%Y%m%dT%H%M%SZ")


def read_source(path: str, what: str, action: str = "") -> Result[str, JudgeError]:
    """The UTF-8 text of a file, or of stdin for ``-``; line endings are normalised to LF. Invalid UTF-8 is INPUT.MALFORMED
    (a typed error, never a crash)."""
    if path == "-":
        try:
            buffer = getattr(sys.stdin, "buffer", None)
            text = buffer.read().decode("utf-8") if buffer is not None else sys.stdin.read()
        except (OSError, ValueError, AttributeError) as exc:  # UnicodeDecodeError is a ValueError; a closed stdin
            return fail("INPUT.MALFORMED", f"{what} on stdin could not be read: {type(exc).__name__}: {exc}",
                        action="Pass UTF-8 text on stdin.")
        return Ok(text.replace("\r\n", "\n"))
    p = pathlib.Path(path) if path else None
    if p is None or not path.strip() or not p.is_file():
        return fail("INPUT.MISSING", f"{what} not found: {path!r}" + (" (not a regular file)" if p is not None and path.strip() and p.exists() else ""),
                    action=action or f"Pass the path of an existing file for the {what}.")
    try:
        return Ok(p.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError) as exc:
        return fail("INPUT.MALFORMED", f"{what} could not be read: {exc}", action=f"Check {path} is a readable UTF-8 file.")


def decode_json(text: str, path: str, what: str) -> Result[Any, JudgeError]:
    """The strict JSON of ``text`` (see strict_loads); INPUT.MALFORMED for anything else."""
    on = " on stdin" if path == "-" else ""
    try:
        return Ok(strict_loads(text))
    except (ValueError, RecursionError) as exc:
        return fail("INPUT.MALFORMED", f"{what}{on} is not valid JSON: {type(exc).__name__}: {exc}",
                    action=f"Fix the JSON in {'the input' if path == '-' else path}.")


def load_json(path: str, what: str) -> Result[Any, JudgeError]:
    text = read_source(path, what)
    if isinstance(text, Err):
        return text
    return decode_json(text.value, path, what)


def content_sha256(text: str) -> str:
    """The digest of a file's text, CRLF as LF, so a Windows checkout (core.autocrlf) gives the same digest as any other."""
    return hashlib.sha256(text.replace("\r\n", "\n").encode("utf-8", "backslashreplace")).hexdigest()


# --------------------------------------------------------------------------- #
# models (ADR-JEV-0005): one strict pydantic model per JSON input; a refusal names the field path and the rule
# --------------------------------------------------------------------------- #
def _non_blank(value: str) -> str:
    if not value.strip():
        raise ValueError("must not be blank")
    return value


def _safe_id_text(value: str) -> str:
    if not BATCH_ID_RE.fullmatch(value):
        raise ValueError("must be letters, digits and _.:- only, starting with a letter or digit, at most 128 characters")
    return value


SafeId = Annotated[str, AfterValidator(_safe_id_text)]
NonBlank = Annotated[str, AfterValidator(_non_blank)]
Probability = Annotated[float, Field(ge=0, le=1, allow_inf_nan=False)]
Choices = Union[str, Annotated[list[str], Field(min_length=1)]]  # a rule's condition value: one choice, or any of several
JsonObjects = Annotated[dict[str, dict[str, Any]], Field(min_length=1)]  # Jev's own question objects, passed through verbatim


class Closed(BaseModel):
    """A format judge-jev owns: wrong types are refused, not coerced, and unknown fields are refused."""

    model_config = ConfigDict(strict=True, extra="forbid")


class Open(BaseModel):
    """A format another tool owns: wrong types are refused, unknown fields are kept."""

    model_config = ConfigDict(strict=True, extra="allow")


# The fields below default to None without allowing it: pydantic does not validate a default, so an absent field is None
# and an explicit null is refused, as the hand-written checks did.
class ChoiceQuestion(Closed):
    type: Literal["choice"]
    instructions: NonBlank
    criteria: Annotated[dict[NonBlank, NonBlank], Field(min_length=2, max_length=255)]  # option name -> description


class NextRule(Closed):
    when: dict[str, Choices] = Field(default_factory=dict)  # question id -> choice or [choices]; every pair must match
    goto: str


class VerdictRule(Closed):
    any_: dict[str, Choices] = Field(default_factory=dict, alias="any")  # matches when any listed question got its choice
    all_: dict[str, Choices] = Field(default_factory=dict, alias="all")  # and every listed question got its choice
    verdict: str


class Leaf(Closed):
    verdict: str


class Wave(Closed):
    """Questions and exactly one of ``next`` (ordered {when, goto}) or ``verdict_rules`` (ordered {any, all, verdict});
    ``per: statement`` sends the wave once per statement of the record field ``statements`` (kept by ``modals``)."""

    questions: Annotated[dict[SafeId, ChoiceQuestion], Field(min_length=1)]
    next: Annotated[list[NextRule], Field(min_length=1)] = None
    verdict_rules: Annotated[list[VerdictRule], Field(min_length=1)] = None
    per: Literal["statement"] = None
    statements: NonBlank = None
    modals: Annotated[list[str], Field(min_length=1)] = None

    @model_validator(mode="after")
    def _rules_and_per(self):
        if (self.next is None) == (self.verdict_rules is None):
            raise ValueError("needs exactly one of next or verdict_rules")
        if self.per is None:
            stray = [name for name in ("statements", "modals") if getattr(self, name) is not None]
            if stray:
                raise ValueError(f"has {' and '.join(stray)} without per")
        elif self.statements is None:
            raise ValueError('per "statement" needs statements, the record field to split')

        def conditions(label: str, when: dict) -> None:
            for qid, wanted in when.items():
                if qid not in self.questions:
                    raise ValueError(f"{label}.{qid}: {qid!r} is not a question of this wave")
                for choice in ([wanted] if isinstance(wanted, str) else wanted):
                    if choice not in self.questions[qid].criteria:
                        raise ValueError(f"{label}.{qid}: {choice!r} is not an option of question {qid!r} "
                                         f"(options: {list(self.questions[qid].criteria)})")

        if self.next is not None:
            for i, rule in enumerate(self.next):
                conditions(f"next[{i}].when", rule.when)
            if self.next[-1].when:
                raise ValueError("the last next rule must have no when, so every combination of answers has an outcome")
        else:
            for i, rule in enumerate(self.verdict_rules):
                conditions(f"verdict_rules[{i}].any", rule.any_)
                conditions(f"verdict_rules[{i}].all", rule.all_)
            if self.verdict_rules[-1].any_ or self.verdict_rules[-1].all_:
                raise ValueError("the last verdict_rules rule must have no any or all, so every combination of answers has an outcome")
        return self


def _node_tag(value) -> str:
    return "wave" if isinstance(value, dict) and "questions" in value else "leaf"  # a node without questions is a leaf


TreeNode = Annotated[Union[Annotated[Wave, Tag("wave")], Annotated[Leaf, Tag("leaf")]], Discriminator(_node_tag)]


def _goto_cycle(nodes: dict):
    """A node that can reach itself through ``next`` rules, or None (the rules are already checked)."""
    def targets(node) -> list:
        return [r.goto for r in (node.next or [])] if isinstance(node, Wave) else []

    color = {}
    for root in nodes:
        if root in color:
            continue
        color[root] = 1
        stack = [(root, iter(targets(nodes[root])))]
        while stack:
            node_id, pending = stack[-1]
            nxt = next(pending, None)
            if nxt is None:
                color[node_id] = 2
                stack.pop()
            elif color.get(nxt) == 1:
                return nxt
            elif nxt not in color:
                color[nxt] = 1
                stack.append((nxt, iter(targets(nodes[nxt]))))
    return None


class QualityTreeModel(Closed):
    """A quality decision tree (assets/questions/quality-<kind>.json): waves and leaves, the walk starting at ``start``."""

    schema_: str = Field(default=None, alias="schema")
    id: str
    kind: Optional[Literal["req", "adr", "nfr"]] = None
    version: int = None
    approved: bool = None
    note: str = None
    start: str
    nodes: Annotated[dict[SafeId, TreeNode], Field(min_length=1)]

    @model_validator(mode="after")
    def _graph(self):
        if self.start not in self.nodes:
            raise ValueError(f"start {self.start!r} is not a node")
        if not isinstance(self.nodes[self.start], Wave):
            raise ValueError(f"start {self.start} is a leaf; the first node must be a wave")
        for node_id, node in self.nodes.items():
            for i, rule in enumerate(node.next or []) if isinstance(node, Wave) else ():
                if rule.goto not in self.nodes:
                    raise ValueError(f"nodes.{node_id}.next[{i}].goto: {rule.goto!r} is not a node")
        looped = _goto_cycle(self.nodes)
        if looped is not None:
            raise ValueError(f"node {looped} can lead back to itself: the tree has a cycle")
        return self


class QuestionSet(Closed):
    """A gate's question set (assets/questions/plan.json, dev.json): Jev's questions and the roles that gate the verdict."""

    schema_: str = Field(default=None, alias="schema")
    gate: str = None
    version: int = None
    notes: str = None
    roles: dict[str, str]
    questions: JsonObjects

    @model_validator(mode="after")
    def _roles_name_questions(self):
        _check_role_map(self.questions, self.roles, REQUIRED_ROLES)
        return self


def _check_role_map(questions: dict, roles: dict, required: tuple) -> None:
    """Every required role, and the optional evidence role if present, names a question of the set (QUESTIONS.ROLES)."""
    for role in required:
        if roles.get(role) not in questions:
            raise PydanticCustomError("roles", "must map role '{role}' to a question id in the set", {"role": role})
    evidence = roles.get("evidence")
    if evidence is not None and evidence not in questions:
        raise PydanticCustomError("roles", "maps role 'evidence' to '{evidence}', which is not a question in the set", {"evidence": evidence})


MAX_TERM, MAX_DEFINITION_TEXT, MAX_SOURCE, MAX_LOCATION = 128, 2000, 512, 512  # characters; a definition is short, never truncated
IRI_TEXT = re.compile(r"[A-Za-z][A-Za-z0-9+.-]+:\S+")  # two or more scheme characters: "C:/x" is a drive path, not an IRI
RECORD_IRI = re.compile(r"[^#\s]+#[^#\s]+")  # <repository URN>#<record id>


def _single_line(value: str) -> str:
    if "\n" in value or "\r" in value:
        raise ValueError("must be one line")
    return value


def _repo_relative(value: str) -> str:
    """A repo-relative path, optionally followed by ``#Heading`` or ``::Symbol``: not absolute, no ``..`` step, no NUL."""
    path = value.partition("::")[0].partition("#")[0]
    steps = path.replace("\\", "/").split("/")
    if "\x00" in value or path.startswith(("/", "\\")) or re.match(r"[A-Za-z]:", path) or ".." in steps or not path.strip():
        raise ValueError("must be a path relative to the repository (no leading slash or drive, no '..')")
    return value


def _source_text(value: str) -> str:
    return value if IRI_TEXT.fullmatch(value) else _repo_relative(value)


def _record_iri_text(value: str) -> str:
    if not RECORD_IRI.fullmatch(value):
        raise ValueError("must be a Raptor record IRI, <repository URN>#<record id> (see jev_runs.record_iri)")
    return value


LocationText = Annotated[str, AfterValidator(_non_blank), AfterValidator(_repo_relative), Field(max_length=MAX_LOCATION)]


class LocationBody(RootModel[LocationText]):
    """A location string (docs/cli.md, Locations): ``path``, ``path#Heading`` or ``path::Symbol``, relative to the repository."""


class AttachmentBody(Closed):
    """One ``jj:Attachment`` of a definition: the record (a Raptor record IRI) and optionally one location."""

    record: Annotated[str, AfterValidator(_record_iri_text), Field(max_length=MAX_SOURCE)]
    location: Optional[LocationText] = None


class DefinitionBody(Closed):
    """The body of a ``jj:Definition`` (ADR-JEV-0008): a non-blank one-line term, a non-blank text, and the source
    (a repo-relative path or a record IRI as a string); the attachments are the graph's edges, checked again here."""

    term: Annotated[str, AfterValidator(_non_blank), AfterValidator(_single_line), Field(max_length=MAX_TERM)]
    text: Annotated[str, AfterValidator(_non_blank), Field(max_length=MAX_DEFINITION_TEXT)]
    source: Annotated[str, AfterValidator(_non_blank), AfterValidator(_source_text), Field(max_length=MAX_SOURCE)]
    attachments: list[AttachmentBody] = []


class DefinitionEntry(Closed):
    """A definition as a request carries it in its state: the term and its text, nothing else."""

    term: Annotated[str, AfterValidator(_non_blank), AfterValidator(_single_line), Field(max_length=MAX_TERM)]
    text: Annotated[str, AfterValidator(_non_blank), Field(max_length=MAX_DEFINITION_TEXT)]


class BatchItem(Open):
    id: SafeId
    title: str = None
    text: NonBlank


class BatchAnchor(Open):
    id: SafeId
    statement: NonBlank
    scope: str = None
    kind: Literal["directive", "prohibition", "structural"] = None


class BatchCandidate(Closed):
    candidate_id: SafeId
    questions: JsonObjects
    roles: dict[str, str] = None
    _effective_roles: dict = PrivateAttr(default_factory=dict)

    @model_validator(mode="after")
    def _roles(self, info):
        """Context: ``anchored`` (the line has an anchor, so the three roles are required) and ``base_roles`` (the gate's)."""
        context = info.context or {}
        anchored = bool(context.get("anchored"))
        roles = self.roles or (context.get("base_roles") or {} if anchored else {})
        if anchored:
            _check_role_map(self.questions, roles, REQUIRED_ROLES)
        self._effective_roles = roles
        return self

    @property
    def effective_roles(self) -> dict:
        return self._effective_roles


class RequestLine(Closed):
    """One line of a batch request stream (assets/schemas/jev-batch-request.schema.json)."""

    item: BatchItem
    anchor: BatchAnchor = None
    location: Annotated[str, AfterValidator(_non_blank), Field(max_length=MAX_LOCATION)] = None
    definitions: Annotated[list[DefinitionEntry], Field(min_length=1)] = None
    questions: Annotated[list[BatchCandidate], Field(min_length=1)]

    @model_validator(mode="after")
    def _distinct_candidates(self):
        seen = set()
        for cand in self.questions:
            if cand.candidate_id in seen:
                raise ValueError(f"candidate {cand.candidate_id} is listed twice (use --repeat for repeats)")
            seen.add(cand.candidate_id)
        return self


def _record_has_id(record: dict) -> dict:
    if not isinstance(record.get("id"), str):
        raise ValueError("needs a string id")
    return record


class RaptorDiagnostics(Open):
    issues: list[dict[str, Any]]


class RaptorIndex(Open):
    """The parts of Raptor's import index (scripts/extract.py --output) that ``quality`` reads; the records are Raptor's and stay open."""

    metadata: dict[str, Any] = None
    requirements: list[Annotated[dict[str, Any], AfterValidator(_record_has_id)]] = None
    decisions: list[Annotated[dict[str, Any], AfterValidator(_record_has_id)]] = None
    diagnostics: RaptorDiagnostics


class ChoiceAnswer(Closed):
    """A Jev choice answer. Context ``options``: the question's option names (the choice and probability keys must be them)."""

    type: Literal["choice"] = "choice"
    choice: str
    confidence: Probability
    probabilities: dict[str, Probability]

    @model_validator(mode="after")
    def _against_the_question(self, info):
        options = (info.context or {}).get("options")
        if options and self.choice not in options:
            raise ValueError(f"choice {self.choice!r} is not one of {sorted(options)}")
        if options and set(self.probabilities) != options:
            raise ValueError("probability keys differ from the options")
        total = sum(self.probabilities.values())
        if abs(total - 1) > PROBABILITY_ROUNDING * max(len(self.probabilities), 1):
            raise ValueError(f"probabilities sum to {total:.4f}")
        return self


class NoulAnswer(Closed):
    """A Jev noul answer: the probability of yes under ``noul``, ``probability`` or ``p`` (the first present is read)."""

    type: Literal["noul"] = None
    noul: Probability = None
    probability: Probability = None
    p: Probability = None

    @model_validator(mode="after")
    def _has_a_probability(self):
        if noul_probability(self.model_dump(exclude_none=True)) is None:
            raise ValueError("noul probability is missing or not in [0, 1]")
        return self


class FixtureError(Closed):
    code: str = Field(alias="__error__", min_length=1, max_length=FIXTURE_CODE_LIMIT)  # BATCH_FIXTURE_ERROR: the attempt fails with this code


def _fixture_problems(entries: dict) -> list:
    """Each envelope problem of an answers file as ``path: rule``. The envelope is what is decidable before any request: an
    entry is an object, {"__error__": code} (a string code of 1 to FIXTURE_CODE_LIMIT characters, nothing else), or a non-empty list of
    those (successive attempts). What an answer object holds (its questions, choices, probabilities) is Jev's response and is
    checked after the send (JEV.RESPONSE_INVALID), so a test can feed an invalid answer on purpose."""
    problems = []
    for key, entry in entries.items():
        attempts = entry if isinstance(entry, list) else [entry]
        if isinstance(entry, list) and not entry:
            problems.append(f"{key}: List should have at least 1 item after validation, not 0")
        for n, attempt in enumerate(attempts):
            where = f"{key}.{n}" if isinstance(entry, list) else key
            if isinstance(attempt, dict) and BATCH_FIXTURE_ERROR in attempt:
                parsed = validate_model(FixtureError, attempt)
                if isinstance(parsed, Err):
                    problems.append(f"{where}: {parsed.error.text}")
            elif not isinstance(attempt, dict):
                problems.append(f"{where}: Input should be an object of answers, an object with {BATCH_FIXTURE_ERROR}, or a list of them")
    return problems


class AnswersFile(RootModel[dict[str, Any]]):
    """The envelope of an answers file, the one model `validate --answers` and every command that reads --answers-file apply
    before the first request: an object of string keys, each an object or a non-empty list of objects, an object with
    {"__error__": code} holding only a string code of 1 to FIXTURE_CODE_LIMIT characters."""

    model_config = ConfigDict(strict=True)

    @model_validator(mode="after")
    def _entries(self):
        problems = _fixture_problems(self.root)
        if problems:
            raise ValueError("; ".join(_clip(p) for p in problems[:MAX_PROBLEMS]) + (f"; and {len(problems) - MAX_PROBLEMS} more" if len(problems) > MAX_PROBLEMS else ""))
        return self


class Refused(NamedTuple):
    """A model refusal: the message (one ``path: rule`` per problem), and the first problem's location and type."""
    text: str
    first_loc: tuple
    first_type: str


def _clip(text: str, limit: int = MAX_MESSAGE) -> str:
    return text if len(text) <= limit else text[:limit - 3] + "..."


def _loc_text(loc: tuple) -> str:
    parts = list(loc)
    if len(parts) > 2 and parts[0] == "nodes" and parts[2] in ("wave", "leaf"):
        del parts[2]  # the union tag pydantic puts after the node id
    return ".".join(str(part) for part in parts)


def validate_model(model, data, context=None) -> Result[Any, Refused]:
    """``data`` as ``model`` (strict), or the refusal naming each failing field path and the rule it breaks."""
    try:
        return Ok(model.model_validate(data, context=context))
    except ValidationError as exc:
        errors = exc.errors(include_url=False, include_context=False, include_input=False)
    except RecursionError:
        return Err(Refused("the input is nested too deeply to check", (), "recursion_error"))
    lines = []
    for error in errors:
        message = error["msg"].removeprefix("Value error, ")
        where = _loc_text(error["loc"])
        line = _clip(f"{where}: {message}" if where else message)
        if line not in lines:
            lines.append(line)
    text = "; ".join(lines[:MAX_PROBLEMS]) + (f"; and {len(lines) - MAX_PROBLEMS} more" if len(lines) > MAX_PROBLEMS else "")
    return Err(Refused(text, tuple(errors[0]["loc"]), errors[0]["type"]))


# --------------------------------------------------------------------------- #
# judging
# --------------------------------------------------------------------------- #
def question_set_error(refused: Refused, where: str) -> JudgeError:
    """A question-set refusal: QUESTIONS.ROLES when a role is the problem, else INPUT.MALFORMED."""
    roles = refused.first_type == "roles" or refused.first_loc[:1] == ("roles",)
    return JudgeError("QUESTIONS.ROLES" if roles else "INPUT.MALFORMED", f"{where}: {refused.text}",
                      action="See references/verdict-gating.md for the role contract." if roles else "See assets/questions/plan.json for the shape.")


def parse_question_set(data, where: str) -> Result[QuestionSet, JudgeError]:
    parsed = validate_model(QuestionSet, data)
    if isinstance(parsed, Err):
        return Err(question_set_error(parsed.error, where))
    return parsed


def load_question_set(gate: str, override_path: str = "") -> Result[QuestionSet, JudgeError]:
    path = override_path or str(QUESTIONS_DIR / f"{gate}.json")
    loaded = load_json(path, f"question set for gate '{gate}'")
    if isinstance(loaded, Err):
        return loaded
    return parse_question_set(loaded.value, f"question set {path}")


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
                      action="Retry with backoff" if transient else "Check the request body.",
                      status=status, retry_after=retry_after)


def call_jev(body: dict, api_key: str, timeout: float) -> Result[dict, JudgeError]:
    """One POST to Jev. Returns Err(JudgeError) on failure; never retries or shortens (judge_with_policy does both)."""
    data = encode_body(body)
    req = urllib.request.Request(
        JEV_URL, data=data, method="POST",
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json", "User-Agent": "judge-jev/jev_judge"},
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read(MAX_RESPONSE_BYTES + 1)
    except urllib.error.HTTPError as exc:
        text = exc.read(4096).decode("utf-8", "replace")
        retry_after = parse_retry_after(exc.headers.get("Retry-After") if exc.headers is not None else None)
        return Err(http_error(exc.code, text, retry_after))
    except urllib.error.URLError as exc:
        return fail("JEV.NETWORK", f"Could not reach Jev: {exc.reason}", recoverable=True, action="Check network access and retry.")
    except (OSError, http.client.HTTPException) as exc:
        return fail("JEV.NETWORK", f"Jev connection failed: {type(exc).__name__}: {exc}", recoverable=True, action="Check network access and retry.")
    if len(raw) > MAX_RESPONSE_BYTES:
        return fail("JEV.RESPONSE_INVALID", "Jev response exceeded the size limit", recoverable=True, action="Retry once; if it repeats, check for a proxy rewriting the response.")
    try:
        parsed = strict_loads(raw.decode("utf-8"))
    except (ValueError, UnicodeError, RecursionError):
        return fail("JEV.RESPONSE_INVALID", "Jev response was not JSON", recoverable=True, action="Retry once; if it repeats, check for a proxy rewriting the response.")
    if not isinstance(parsed, dict):
        return fail("JEV.RESPONSE_INVALID", "Jev response was not a JSON object", recoverable=True, action="Check for a proxy rewriting the response.")
    return Ok(parsed)


def is_transient(exc: JudgeError) -> bool:
    if exc.code in TRANSIENT_CODES:
        return True
    return exc.code == "JEV.HTTP" and exc.status is not None and exc.status >= 500


def is_probability(value) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and 0 <= value <= 1


def noul_probability(ans: dict):
    for key in ("noul", "probability", "p"):
        if key in ans:
            return ans[key]
    return None


def is_noul(ans: dict) -> bool:
    """A noul answer: type "noul", or no type and a "noul" key. answer_problem and answer_choice both use this."""
    return ans.get("type") == "noul" or ("type" not in ans and "noul" in ans)


def answer_problem(ans, question) -> Optional[str]:
    """Why an answer cannot be gated, or None when it can: the answer parsed as the choice or noul model, the choice and
    probabilities checked against the question's options."""
    if not isinstance(ans, dict):
        return "answer is not an object"
    kind = "noul" if is_noul(ans) else ans.get("type", "choice")
    if kind not in ("choice", "noul"):
        return f"answer type {kind!r} is not choice or noul"
    criteria = question.get("criteria") if isinstance(question, dict) else None
    options = set(criteria) if isinstance(criteria, dict) else set()
    parsed = validate_model(NoulAnswer if kind == "noul" else ChoiceAnswer, ans, {"options": options})
    return None if isinstance(parsed, Ok) else parsed.error.text


def validate_response(resp: dict, questions: dict, optional=frozenset()) -> Result[dict, JudgeError]:
    """Every question must be answered except the ``optional`` (extra, never-gating) ones; no unknown ids."""
    if not isinstance(resp, dict) or not isinstance(resp.get("answers"), dict):
        return fail("JEV.RESPONSE_INVALID", "Jev response has no 'answers' object", recoverable=True,
                    action="Retry; if it repeats, check the --model name.")
    answers = resp["answers"]
    required = set(questions) - set(optional)
    if not required <= set(answers) or not set(answers) <= set(questions):
        missing, extra = sorted(required - set(answers)), sorted(set(answers) - set(questions))
        return fail("JEV.RESPONSE_INVALID", f"Jev answer ids differ from the questions (missing {missing}, unexpected {extra})", recoverable=True,
                    action="Retry; if it repeats, check the question ids in the overrides.")
    for qid, q in questions.items():
        if qid not in answers:
            continue  # an optional question left unanswered
        problem = answer_problem(answers[qid], q)
        if problem:
            return fail("JEV.RESPONSE_INVALID", _clip(f"invalid answer for {qid}: {problem}", ATTEMPT_MESSAGE_LIMIT), recoverable=True,
                        action="Retry; if it repeats, the question set or the model changed shape.")
    return Ok(resp)


def answer_choice(ans: dict) -> tuple:
    """Return (choice, confidence) for a choice answer; tolerate noul answers."""
    if is_noul(ans):  # answer_problem checked its probability
        p = float(noul_probability(ans) or 0.0)
        return ("yes" if p >= 0.5 else "no", abs(p - 0.5) * 2)
    return (ans.get("choice", "unclear"), float(ans.get("confidence", 0.0)))


def role_answers(answers: dict, roles: dict) -> dict:
    """{role: (choice, confidence)} for every role that maps to an answered question."""
    out = {}
    for role, qid in (roles or {}).items():
        if qid and qid in answers:
            out[role] = answer_choice(answers[qid])
    return out


def decide(answers: dict, roles: dict, threshold: float, kind: str = "directive") -> tuple:
    """Turn independent Jev answers into one verdict, gating in code (Jev cannot see its own answers).

    directive (and structural at the dev gate): relevance, compliance and conflict all gate, as in the table in
    references/verdict-gating.md. prohibition: the compliance role is ignored, because the default `addressed`
    question answers no for every bead that simply refrains (10 of 10 beads in the measured experiments): not_applicable
    when relevance is no, contradicted when conflict is yes, underspecified when relevance is unclear/low or the
    conflict answer is not a confident no, else satisfied (the dev gate's evidence role still yields asserted_only).
    With threshold 0 the result is the verdict by choices alone: the `leaning`.
    """
    got = role_answers(answers, roles)

    def get(role):
        return got.get(role, (None, 0.0))

    rel, rel_c = get("relevance")
    comp, comp_c = get("compliance")
    conf, conf_c = get("conflict")
    evid, evid_c = get("evidence")

    if kind == "prohibition":
        min_conf = min(rel_c, conf_c)
        if rel == "no" and rel_c >= threshold:
            return "not_applicable", min_conf
        if conf == "yes" and conf_c >= threshold:
            return "contradicted", min_conf
        if rel == "unclear" or rel_c < threshold or conf != "no" or conf_c < threshold:
            return "underspecified", min_conf
        if roles.get("evidence") and evid == "no" and evid_c >= threshold:
            return "asserted_only", min_conf
        return "satisfied", min_conf

    min_conf = min(rel_c, comp_c, conf_c)
    if rel == "no" and rel_c >= threshold:
        return "not_applicable", min_conf
    if conf == "yes" and conf_c >= threshold:
        return "contradicted", min_conf
    if rel == "unclear" or rel_c < threshold or comp == "unclear" or comp_c < threshold:
        return "underspecified", min_conf
    if comp == "no":
        return "not_addressed", min_conf
    if comp == "yes" and conf == "no" and conf_c >= threshold:  # an unsure conflict answer never passes
        if roles.get("evidence") and evid == "no" and evid_c >= threshold:
            return "asserted_only", min_conf
        return "satisfied", min_conf
    return "underspecified", min_conf


def _command_credential(argv: list[str]) -> str:
    """Read a credential from a platform command without exposing its output on failure."""
    try:
        proc = subprocess.run(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                              text=True, encoding="utf-8", timeout=5, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return ""
    return proc.stdout.strip() if proc.returncode == 0 else ""


def _windows_credential() -> str:
    """Read the documented generic credential target from Windows Credential Manager."""
    try:
        import ctypes
        from ctypes import wintypes

        class Credential(ctypes.Structure):
            _fields_ = [
                ("Flags", wintypes.DWORD), ("Type", wintypes.DWORD), ("TargetName", wintypes.LPWSTR),
                ("Comment", wintypes.LPWSTR), ("LastWritten", wintypes.FILETIME),
                ("CredentialBlobSize", wintypes.DWORD), ("CredentialBlob", ctypes.c_void_p),
                ("Persist", wintypes.DWORD), ("AttributeCount", wintypes.DWORD), ("Attributes", ctypes.c_void_p),
                ("TargetAlias", wintypes.LPWSTR), ("UserName", wintypes.LPWSTR),
            ]

        credential = ctypes.POINTER(Credential)()
        advapi = ctypes.windll.advapi32
        advapi.CredReadW.argtypes = (wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
                                     ctypes.POINTER(ctypes.POINTER(Credential)))
        advapi.CredReadW.restype = wintypes.BOOL
        advapi.CredFree.argtypes = (ctypes.c_void_p,)
        if not advapi.CredReadW(WINDOWS_CREDENTIAL_TARGET, 1, 0, ctypes.byref(credential)):
            return ""
        try:
            size = int(credential.contents.CredentialBlobSize)
            if not size or not credential.contents.CredentialBlob:
                return ""
            return ctypes.string_at(credential.contents.CredentialBlob, size).decode("utf-16-le").strip()
        finally:
            advapi.CredFree(credential)
    except (AttributeError, OSError, UnicodeDecodeError):
        return ""


def credential_store_api_key() -> str:
    """Read the documented OS credential only when the environment has no key."""
    if sys.platform == "darwin":
        return _command_credential(["security", "find-generic-password", "-s", CREDENTIAL_SERVICE,
                                    "-a", CREDENTIAL_ACCOUNT, "-w"])
    if sys.platform == "win32":
        return _windows_credential()
    if sys.platform.startswith("linux"):
        return _command_credential(["secret-tool", "lookup", "service", CREDENTIAL_SERVICE,
                                    "account", CREDENTIAL_ACCOUNT])
    return ""


def resolved_api_key() -> str:
    """Resolve the TypeSafe key from the environment first, then the platform credential store."""
    environment_key = os.environ.get(TYPESAFE_KEY_ENV)
    if environment_key is not None:
        return environment_key.strip()
    return credential_store_api_key()


def check_api_key(api_key: str) -> Result[None, JudgeError]:
    if not api_key:
        return fail("AUTH.CREDENTIALS", "no TypeSafe API key was found", action="Export TYPESAFE_API_KEY or add the documented platform credential, or use --dry-run / --answers-file.")
    if not api_key.isascii() or any(ord(c) <= 32 or ord(c) == 127 for c in api_key):
        return fail("AUTH.CREDENTIALS", "the TypeSafe API key has invalid formatting (whitespace, control or non-ASCII characters)",
                    action="Replace TYPESAFE_API_KEY or the platform credential with the key exactly as issued.")
    return Ok(None)


REMOTE_JUDGE_SETTING = ".sc/judge-jev/settings.yaml"
REPO_ROOT_HELP = ("any path inside the repository; the nearest .sc/judge-jev/settings.yaml at or above it holds "
                  "allow_remote_judge (read for live Jev calls only; default .)")
REMOTE_JUDGE_SET = "python3 scripts/judge_jev_settings.py set allow_remote_judge true|false"


def remote_judge_consent(args, what: str) -> Result[None, JudgeError]:
    """The repository's committed answer to whether record text may go to TypeSafe: ``allow_remote_judge`` in
    .sc/judge-jev/settings.yaml. Only ``true`` passes; ``false`` and an absent answer refuse.

    The settings module (and PyYAML behind it) is imported only here, so only a live call needs it.
    """
    import judge_jev_settings as settings  # lazy: dry runs and fixture answers stay stdlib-only

    start = pathlib.Path(getattr(args, "repository_root", "") or ".")
    root = settings.find_repo_root(start)  # consent is a repository's answer: outside one there is none
    if not root.ok:
        return Err(_from_tool_error(root.error))
    # The nearest settings file at or above the given path wins (REQ-070), not the one at the git top level:
    # an install in a folder inside the repository writes its settings there.
    answer = settings.remote_judge_consent(start.resolve())
    if answer.ok and answer.value is True:
        return Ok(None)
    disabled = f"Sending record text to TypeSafe is disabled. {what} would leave this machine."
    if answer.ok:  # False: the user said no
        return fail("POLICY.REMOTE_JUDGE_DISABLED",
                    f"{disabled} {REMOTE_JUDGE_SETTING} sets allow_remote_judge: false.",
                    action="The user declined remote judging; only the user changes it (`python3 scripts/judge_jev_settings.py "
                           "set allow_remote_judge true`, then commit). Use --dry-run to inspect the request instead.")
    if answer.error.code == "SETTINGS.REMOTE_JUDGE_UNSET":
        return fail("POLICY.REMOTE_JUDGE_DISABLED",
                    f"{disabled} {REMOTE_JUDGE_SETTING} does not set allow_remote_judge.",
                    action="Ask the user once whether record text may be sent to TypeSafe (Jev) for judging, record the answer "
                           f"with `{REMOTE_JUDGE_SET}`, and commit {REMOTE_JUDGE_SETTING}. Never infer consent from "
                           "existing run files or past runs. Use --dry-run to inspect the request instead.")
    return Err(_from_tool_error(answer.error))  # DEPENDENCY.PYYAML, SETTINGS.*: never a silent default


def _from_tool_error(error) -> JudgeError:
    out = JudgeError(error.code, error.message, action=error.action or "")
    out.data = dict(error.data) if error.data is not None else None
    return out


def remote_preflight(args, what: str) -> Result[str, JudgeError]:
    """Key and consent checks shared by every live Jev call; returns the API key. Nothing leaves the machine before this passes."""
    api_key = resolved_api_key()
    checked = check_api_key(api_key)
    if isinstance(checked, Err):
        return checked
    consent = remote_judge_consent(args, what)
    if isinstance(consent, Err):
        return consent
    return Ok(api_key)


def load_answers_fixture(path: str, keyed_by: str) -> Result[Any, JudgeError]:
    """A fixture of Jev answers keyed by id (or '*'); Ok(None) when no path is given."""
    if not path:
        return Ok(None)
    loaded = load_json(path, "answers fixture")
    if isinstance(loaded, Err):
        return loaded
    fixture = loaded.value
    if not isinstance(fixture, dict):
        return fail("INPUT.MALFORMED", f"answers fixture must be an object keyed by {keyed_by} (or '*')")
    enveloped = answers_envelope_error(fixture, f"answers fixture {path}")
    if enveloped is not None:
        return Err(enveloped)
    return Ok(fixture)


def answers_envelope_error(data, where: str) -> Optional[JudgeError]:
    """None when ``data`` is a valid answers-file envelope (AnswersFile), else INPUT.MALFORMED naming the field path; the one
    check behind `validate --answers` and --answers-file."""
    parsed = validate_model(AnswersFile, data)
    if isinstance(parsed, Ok):
        return None
    return JudgeError("INPUT.MALFORMED", f"{where}: {parsed.error.text}",
                      action="An answers file maps a request key to Jev's answers (an object), a non-empty list of them, or "
                             "an __error__ object with a string code of 1 to " + str(FIXTURE_CODE_LIMIT) + " characters and nothing else.")


def check_threshold(value: float, name: str = "--threshold") -> Result[float, JudgeError]:
    if not (0.0 <= value <= 1.0):
        return fail("VALIDATION.INPUT", f"{name} must be between 0 and 1, got {value}")
    return Ok(value)


# --------------------------------------------------------------------------- #
# cli
# --------------------------------------------------------------------------- #
# --------------------------------------------------------------------------- #
# batch (low layer, tracker-neutral): request lines in, one result line per (item, candidate, repeat) out
# --------------------------------------------------------------------------- #
BATCH_RESULT_SCHEMA = "judge-jev/jev-batch-result/1"
BATCH_MAX_WORKERS = 4              # parallel Jev evaluators; the anchor-tuning protocol's cap
BATCH_RETRIES = 1                  # one retry per failed call, recorded as retry_count on the line
BATCH_FIXTURE_ERROR = "__error__"  # an answers-file attempt {"__error__": "<code>"} simulates a failed call


class BatchSettings(NamedTuple):
    gate: str              # plan | dev: the default roles when a candidate carries none
    model: str
    repeat: int
    prefix: str            # request ids are <prefix>/<item>/<anchor>/<candidate>/r<n>
    threshold: float       # the line's verdict; leaning is the same answers at threshold 0
    timeout: float
    max_workers: int
    max_request_bytes: int
    dry_run: bool


class BatchJob(NamedTuple):
    item_id: str
    anchor_id: Optional[str]  # None for a request line without an anchor (a quality wave)
    candidate_id: str
    repeat: int
    request_id: str
    body: dict
    questions: dict
    roles: dict
    kind: str


def batch_request_id(prefix: str, item_id: str, anchor_id, candidate_id: str, repeat: int) -> str:
    if anchor_id is None:  # a request line without an anchor
        return f"{prefix}/{item_id}/{candidate_id}/r{repeat}"
    return f"{prefix}/{item_id}/{anchor_id}/{candidate_id}/r{repeat}"


def batch_run_id(request_id: str) -> str:
    """Deterministic run id: the first 16 hex digits of SHA-256 over the request id."""
    return hashlib.sha256(request_id.encode("utf-8")).hexdigest()[:16]


def _safe_id(value, what: str) -> Result[str, JudgeError]:
    if not isinstance(value, str) or not BATCH_ID_RE.fullmatch(value):
        return fail("VALIDATION.INPUT", f"{what} {value!r} is not a safe id (letters, digits and _.:- only)",
                    action="See assets/schemas/jev-batch-request.schema.json.")
    return Ok(value)


def request_line_error(refused: Refused, where: str) -> JudgeError:
    """A request-line refusal: QUESTIONS.ROLES for a role problem, INPUT.MALFORMED for a candidate's question set, else
    VALIDATION.INPUT."""
    loc = refused.first_loc
    if refused.first_type == "roles":
        code = "QUESTIONS.ROLES"
    elif loc[:1] == ("questions",) and len(loc) >= 3 and loc[2] == "questions":
        code = "INPUT.MALFORMED"
    else:
        code = "VALIDATION.INPUT"
    return JudgeError(code, f"{where}: {refused.text}", action="See assets/schemas/jev-batch-request.schema.json.")


def batch_jobs(line, number: int, settings: BatchSettings, base_roles: dict, log_fields=None) -> Result[list, JudgeError]:
    """One request line -> its jobs (candidates x repeats, in order). Refuses a malformed line, an oversized request, or a
    request whose log events the logger would refuse, before any call (``log_fields``: the fields the caller adds to every
    event of the line).

    A line without ``anchor`` (a quality wave: the record is the item) sends only the item, needs no roles and gets no
    verdict; a line with one is gated exactly as before."""
    where = f"request line {number}"
    anchored = isinstance(line, dict) and "anchor" in line
    parsed = validate_model(RequestLine, line, {"anchored": anchored, "base_roles": base_roles})
    if isinstance(parsed, Err):
        return Err(request_line_error(parsed.error, where))
    request = parsed.value
    item, anchor = request.item, request.anchor
    jobs = []
    item_state = {"id": item.id, "title": item.title or "", "text": item.text}
    state = build_state({"id": anchor.id, "statement": anchor.statement, "scope": anchor.scope}, item_state) if anchored else {"item": item_state}
    if request.location is not None:
        state["location"] = request.location
    if request.definitions is not None:  # ADR-JEV-0008: absent when nothing matched, so such a request is as before
        state["definitions"] = [{"term": d.term, "text": d.text} for d in request.definitions]
    kind = anchor.kind or "directive" if anchored else "directive"
    anchor_id = anchor.id if anchored else None
    for cand in request.questions:
        body = {"model": settings.model, "state": state, "questions": cand.questions}
        size = len(encode_body(body))
        if size > settings.max_request_bytes:
            return fail("JEV.CONTEXT_EXCEEDED", f"{where}: the request for {item.id}/{cand.candidate_id} is {size} bytes, over --max-request-bytes {settings.max_request_bytes}",
                        action="Shorten the item (see references/plan-size-and-shape.md); never raise the cap to make it fit.")
        first = len(jobs)
        for r in range(1, settings.repeat + 1):
            rid = batch_request_id(settings.prefix, item.id, anchor_id, cand.candidate_id, r)
            jobs.append(BatchJob(item.id, anchor_id, cand.candidate_id, r, rid, body, cand.questions, cand.effective_roles, kind))
        checked = check_log_events(jobs[first:][-1], settings, f"{where}: {item.id}/{cand.candidate_id}", log_fields or {})
        if isinstance(checked, Err):
            return checked
    return Ok(jobs)


def _fixture_keys(job: BatchJob) -> tuple:
    return (f"{job.item_id}/{job.candidate_id}/r{job.repeat}", f"{job.item_id}/{job.candidate_id}", job.candidate_id, "*")


def _fixture_missing(keys: tuple) -> Err:
    return fail("FIXTURE.MISSING", f"answers fixture has no entry for {keys[0]} (or {keys[1]}, {keys[2]}, '*')",
                action=f"Add an entry for {keys[0]} to the answers file.")


def check_fixture_coverage(lines: list, settings: BatchSettings, fixture) -> Result[None, JudgeError]:
    """FIXTURE.MISSING for the first request of ``lines`` the answers fixture has no entry (and no ``*``) for, before any is
    sent. Ok when there is no fixture or the run is a dry run (it consults none)."""
    if fixture is None or settings.dry_run:
        return Ok(None)
    base = load_question_set(settings.gate)
    if isinstance(base, Err):
        return base
    for number, line in enumerate(lines, start=1):
        jobs = batch_jobs(line, number, settings, base.value.roles)
        if isinstance(jobs, Err):
            return jobs
        for job in jobs.value:
            keys = _fixture_keys(job)
            if not any(k in fixture for k in keys):
                return _fixture_missing(keys)
    return Ok(None)


def _fixture_attempt(job: BatchJob, fixture: dict, attempt: int) -> Result[dict, JudgeError]:
    keys = _fixture_keys(job)
    entry = next((fixture[k] for k in keys if k in fixture), None)
    if entry is None:
        return _fixture_missing(keys)
    if isinstance(entry, list):
        if not entry:
            return fail("FIXTURE.MISSING", f"answers fixture entry for {keys[0]} is an empty list")
        entry = entry[min(attempt, len(entry)) - 1]
    if isinstance(entry, dict) and BATCH_FIXTURE_ERROR in entry:
        code = str(entry[BATCH_FIXTURE_ERROR])
        return fail(code, f"fixture failure {code}", recoverable=code in TRANSIENT_CODES)
    return Ok({"model": "fixture", "answers": entry})


class _AbortFlag:
    """Set once by the first call that must stop the batch (AUTH.CREDENTIALS, POLICY.*); later jobs send nothing."""

    def __init__(self):
        self._event = threading.Event()
        self.error = None

    def set(self, error: JudgeError) -> None:
        self.error = error
        self._event.set()

    def is_set(self) -> bool:
        return self._event.is_set()


def _line(job: BatchJob, settings: BatchSettings, raw, attempts: list, error, dry_run: bool) -> dict:
    verdict = min_conf = leaning = None
    if raw is not None and job.anchor_id is not None:  # a line without an anchor has no gate: its caller reads the answers
        verdict, min_conf = decide(raw["answers"], job.roles, settings.threshold, job.kind)
        leaning = decide(raw["answers"], job.roles, 0.0, job.kind)[0]
    return redact_sensitive({
        "schema": BATCH_RESULT_SCHEMA, "request_id": job.request_id, "run_id": batch_run_id(job.request_id),
        "item_id": job.item_id, "anchor_id": job.anchor_id, "candidate_id": job.candidate_id, "repeat": job.repeat,
        "request_sha256": hashlib.sha256(encode_body(job.body)).hexdigest(), "questions_sha256": questions_digest(job.questions),
        "request": job.body, "roles": job.roles, "kind": job.kind, "dry_run": dry_run, "ok": raw is not None,
        "raw_response": raw, "answers": raw["answers"] if raw is not None else None,
        "model_answered": raw.get("model") if raw is not None else None,
        "threshold": settings.threshold, "verdict": verdict, "min_confidence": min_conf, "leaning": leaning,
        "retry_count": max(len(attempts) - 1, 0), "attempts": attempts,
        "error": None if error is None else {"code": error.code, "message": _clip(error.message, ATTEMPT_MESSAGE_LIMIT), "recoverable": error.recoverable},
    })


def run_batch_job(job: BatchJob, settings: BatchSettings, api_key: str, fixture, abort: _AbortFlag) -> dict:
    """One call with one retry on a recoverable failure; returns the result line (raw evidence, redacted)."""
    if settings.dry_run:
        return _line(job, settings, None, [], None, True)
    attempts, raw, error = [], None, None
    for attempt in range(1, BATCH_RETRIES + 2):
        if abort.is_set():
            error = abort.error
            attempts.append({"attempt": attempt, "ok": False, "error": {"code": error.code, "message": "not sent: the batch aborted", "recoverable": False}})
            break
        got = _fixture_attempt(job, fixture, attempt) if fixture is not None else call_jev(job.body, api_key, settings.timeout)
        if isinstance(got, Ok):
            got = validate_response(got.value, job.questions)
        if isinstance(got, Ok):
            raw, error = got.value, None
            attempts.append({"attempt": attempt, "ok": True})
            break
        error = got.error
        attempts.append({"attempt": attempt, "ok": False, "error": {"code": error.code, "message": _clip(error.message, ATTEMPT_MESSAGE_LIMIT), "recoverable": error.recoverable}})
        if error.code in ABORT_CODES:
            abort.set(error)
            break
        if not (error.recoverable or is_transient(error)) or attempt > BATCH_RETRIES:
            break
        if fixture is None:
            _sleep(error.retry_after if error.retry_after is not None else RETRY_DELAYS[0])
    return _line(job, settings, raw, attempts, error, False)


async def _aiter(source):
    if hasattr(source, "__aiter__"):
        async for value in source:
            yield value
    else:
        for value in source:
            yield value


class BatchInputError(Exception):
    """A request line the async API cannot judge; carries the typed JudgeError. The CLI validates every line first, so it never sees one."""

    def __init__(self, error: JudgeError):
        super().__init__(error.message)
        self.error = error


async def stream_batch(source, settings: BatchSettings, api_key: str = "", fixture=None):
    """Library API: an async iterator of result lines for an (async) iterable of request-line dicts.

    At most ``settings.max_workers`` Jev calls run at once. Lines are yielded in request order (candidates,
    then repeats, within each request line) as soon as each one and every line before it has an answer, so
    the stream is deterministic for the same input; consumers still key lines by id, never by position.
    Raises BatchInputError for an invalid request line.
    """
    loop = asyncio.get_running_loop()
    base = load_question_set(settings.gate)
    if isinstance(base, Err):
        raise BatchInputError(base.error)
    abort = _AbortFlag()
    pending = collections.deque()
    window = settings.max_workers * 2
    with concurrent.futures.ThreadPoolExecutor(max_workers=settings.max_workers) as pool:
        number = 0
        async for line in _aiter(source):
            number += 1
            jobs = batch_jobs(line, number, settings, base.value.roles)
            if isinstance(jobs, Err):
                raise BatchInputError(jobs.error)
            for job in jobs.value:
                pending.append(loop.run_in_executor(pool, run_batch_job, job, settings, api_key, fixture, abort))
                while len(pending) >= window:
                    yield await pending.popleft()
        while pending:
            yield await pending.popleft()


def parse_request_lines(text: str, path: str) -> Result[list, JudgeError]:
    """Request lines from the text of a request stream: one JSON object per non-empty line."""
    lines = []
    for n, line in enumerate(text.splitlines(), start=1):
        if not line.strip():
            continue
        try:
            lines.append(strict_loads(line))
        except (ValueError, RecursionError) as exc:
            return fail("INPUT.MALFORMED", f"request line {n} is not JSON: {type(exc).__name__}: {exc}", action="Write one JSON object per line.")
    if not lines:
        return fail("INPUT.EMPTY", "the request stream has no lines")
    return Ok(lines)


def read_request_lines(path: str) -> Result[list, JudgeError]:
    """Request lines from a file or stdin (``-``): the same parser for both, one JSON object per non-empty line."""
    raw = read_source(path, "request stream", action="Pass a JSONL file, or - for stdin.")
    if isinstance(raw, Err):
        return raw
    return parse_request_lines(raw.value, path)


def write_stdout_line(text: str) -> None:
    """One JSONL line to stdout, byte for byte as written to the log: through the binary buffer, because a text-mode
    stdout on Windows turns "\n" into "\r\n". A stream without a buffer (a test's StringIO) gets the text."""
    buffer = getattr(sys.stdout, "buffer", None)
    if buffer is None:
        sys.stdout.write(text)
        sys.stdout.flush()
        return
    sys.stdout.flush()
    buffer.write(text.encode("utf-8"))
    buffer.flush()


def encode_result_line(line: dict) -> str:
    return json.dumps(line, ensure_ascii=False) + "\n"  # key order kept: request_sha256 hashes the request as sent


def batch_settings(args, prefix: str) -> Result[BatchSettings, JudgeError]:
    if getattr(args, "repeat", 1) < 1:
        return fail("VALIDATION.INPUT", "--repeat must be at least 1")
    if not 1 <= args.max_workers <= BATCH_MAX_WORKERS:
        return fail("VALIDATION.INPUT", f"--max-workers must be between 1 and {BATCH_MAX_WORKERS}")
    if not prefix.strip() or any(c.isspace() for c in prefix):
        return fail("VALIDATION.INPUT", "--request-prefix must be non-empty and contain no whitespace")
    threshold = check_threshold(getattr(args, "threshold", DEFAULT_THRESHOLD))
    if isinstance(threshold, Err):
        return threshold
    gate = getattr(args, "gate", "plan")
    cap = request_cap(gate, getattr(args, "max_request_bytes", None))
    if isinstance(cap, Err):
        return cap
    return Ok(BatchSettings(gate, args.model, getattr(args, "repeat", 1), prefix, threshold.value, args.timeout,
                            args.max_workers, cap.value, bool(args.dry_run)))


def request_cap(gate: str, requested) -> Result[int, JudgeError]:
    """The request size limit: ``requested`` or the gate's default, at most MAX_REQUEST_BYTES_CEILING."""
    cap = requested or (DEFAULT_MAX_REQUEST_BYTES if gate == "plan" else DEFAULT_MAX_REQUEST_BYTES_DEV)
    if cap > MAX_REQUEST_BYTES_CEILING:
        return fail("VALIDATION.INPUT", f"--max-request-bytes {cap} is over {MAX_REQUEST_BYTES_CEILING}: every request is logged whole "
                    "and the run log refuses events over 64 KiB",
                    action=f"Pass --max-request-bytes {MAX_REQUEST_BYTES_CEILING} or less, or shorten the item text.")
    return Ok(cap)


LOG_SERVICE = "judge-jev"
LOG_TARGET = "judge-jev.batch"
SC_OBSERVABILITY_SPEC = "sc-observability==1.5.0"


class RunLog:
    """The run log (REQ-JEV-0008): one owned sc-observability logger per command, writing under
    ``<log_root>/logs/judge-jev.log.jsonl``. Every event carries the run name plus the caller's ``fields``."""

    def __init__(self, sco, logger, run: str, log_root: pathlib.Path, size_at_open: int = 0):
        self._sco = sco
        self._logger = logger
        self.run = run
        self.log_root = log_root
        self._size_at_open = size_at_open
        self._events = 0

    @property
    def path(self) -> pathlib.Path:
        return self.log_root / "logs" / f"{LOG_SERVICE}.log.jsonl"

    def event(self, action: str, fields: dict, correlation_id=None) -> Result[None, JudgeError]:
        """One log event, redacted like every emitted value; anything but an accepted admission is LOG.WRITE."""
        got = self._logger.log(self._sco.LogEvent(level="info", target=LOG_TARGET, action=action,
                                                  correlation_id=correlation_id,
                                                  fields=redact_sensitive({"run": self.run, **fields})))
        if isinstance(got, self._sco.Err) or got.value.kind != "accepted":
            detail = getattr(got.error, "message", got.error) if isinstance(got, self._sco.Err) else got.value.kind
            return fail("LOG.WRITE", f"the sc-observability logger refused the {action} event: {detail}",
                        action="Read the message; the event was not recorded, so rerun once the cause is fixed.")
        self._events += 1
        return Ok(None)

    def flush(self) -> Result[None, JudgeError]:
        got = self._logger.flush()
        if isinstance(got, self._sco.Err):
            return fail("LOG.WRITE", f"the sc-observability logger could not flush: {getattr(got.error, 'message', got.error)}")
        return Ok(None)

    def close(self) -> Result[None, JudgeError]:
        got = self._logger.shutdown()
        if isinstance(got, self._sco.Err):
            return fail("LOG.WRITE", f"the sc-observability logger could not shut down: {getattr(got.error, 'message', got.error)}")
        try:  # the logger admits events it later fails to write, so check the file grew
            grew = self.path.stat().st_size > self._size_at_open
        except OSError:
            grew = False
        if self._events and not grew:
            return fail("LOG.WRITE", f"{self._events} event(s) were admitted but {self.path} did not grow; the run is not logged",
                        action="Check the run directory is a writable directory, then rerun.")
        return Ok(None)


def logger_module():
    """The sc-observability module (imported lazily: only a command that logs, or sizes a log event, needs it)."""
    try:
        import sc_observability as sco
    except ImportError as exc:
        return fail("DEPENDENCY.SC_OBSERVABILITY", f"the sc-observability logger is required for the run log and did not import: {exc}",
                    action=f"Run with `uv run --script jev_judge.py ...` (its inline metadata pins {SC_OBSERVABILITY_SPEC}), "
                           f"or python3 -m pip install '{SC_OBSERVABILITY_SPEC}' (Python 3.11+).")
    return Ok(sco)


def open_run_log(log_root, run_name: str) -> Result[RunLog, JudgeError]:
    """Start the run's logger; the run name defaults to a UTC timestamp and must be a safe id."""
    name = run_name or stamp()
    checked = _safe_id(name, "--run-name")
    if isinstance(checked, Err):
        return checked
    module = logger_module()
    if isinstance(module, Err):
        return module
    sco = module.value
    root = pathlib.Path(log_root)
    log_file = root / "logs" / f"{LOG_SERVICE}.log.jsonl"
    try:  # before any call: the log file must be appendable
        log_file.parent.mkdir(parents=True, exist_ok=True)
        with open(log_file, "ab"):
            pass
        size_at_open = log_file.stat().st_size
    except OSError as exc:
        return fail("LOG.WRITE", f"the run log {log_file} is not writable: {exc}",
                    action="Pass a --run-dir that is (or can be created as) a writable directory.")
    created = sco.create_logger(sco.LoggerConfig(service=LOG_SERVICE, log_root=str(root)))
    if isinstance(created, sco.Err):
        return fail("LOG.WRITE", f"the sc-observability logger could not start under {root}: {getattr(created.error, 'message', created.error)}",
                    action="Check the run directory is writable.")
    return Ok(RunLog(sco, created.value, name, root, size_at_open))


def result_line_events(line: dict, fields: dict) -> list:
    """A result line as three events: the request, Jev's answer (not on a dry run) and the result (the line without its
    request and raw response). ``fields`` goes on every event."""
    ids = {"request_id": line["request_id"], "run_id": line["run_id"], **fields}
    events = [("request", {**ids, "item_id": line["item_id"], "anchor_id": line["anchor_id"], "candidate_id": line["candidate_id"],
                           "repeat": line["repeat"], "request_sha256": line["request_sha256"],
                           "questions_sha256": line["questions_sha256"], "dry_run": line["dry_run"], "request": line["request"]})]
    if not line["dry_run"]:
        events.append(("answer", {**ids, "ok": line["ok"], "model_answered": line["model_answered"], "raw_response": line["raw_response"],
                                  "attempts": line["attempts"], "error": line["error"]}))
    events.append(("result", {**ids, **{k: v for k, v in line.items() if k not in ("request", "raw_response")}}))
    return events


def log_result_line(log: RunLog, line: dict, fields: dict) -> Result[None, JudgeError]:
    """A result line as its events (result_line_events), flushed. The logger refuses an event over its size limit; every
    event was sized before the request was sent (check_log_events)."""
    for action, event_fields in result_line_events(line, fields):
        logged = log.event(action, event_fields, correlation_id=line["run_id"])
        if isinstance(logged, Err):
            return logged
    return log.flush()


# The size of a log event is measured the way the sc-observability logger measures it (sc-observability 1.5.0,
# Logger.log): the event's fields encoded as typed values (sc_observability._event), serialised as compact ASCII JSON, in
# UTF-8 bytes, against AdapterPolicy().max_request_bytes. test_log_event_limit pins this to the logger's own refusal.
LOG_RUN_NAME_WORST = "r" * 128  # a run name is a safe id: at most 128 characters


def log_event_size(sco, action: str, fields: dict, correlation_id=None) -> Result[int, str]:
    """The bytes the logger counts for one event, or the logger's own reason it cannot encode the event (a nesting depth)."""
    event = sco.LogEvent(level="info", target=LOG_TARGET, action=action, correlation_id=correlation_id,
                         fields=redact_sensitive({"run": LOG_RUN_NAME_WORST, **fields}))
    encoded = sco._event(event)
    if isinstance(encoded, sco.Err):
        return Err(str(getattr(encoded.error, "message", encoded.error)))
    return Ok(len(json.dumps(encoded.value, separators=(",", ":")).encode("utf-8")))


def check_event_sizes(events: list, where: str) -> Result[None, JudgeError]:
    """Each (action, fields, correlation_id) event fits the logger's limit; else the event and its size are named."""
    module = logger_module()
    if isinstance(module, Err):
        return module
    sco = module.value
    limit = sco.AdapterPolicy().max_request_bytes
    for action, fields, correlation_id in events:
        size = log_event_size(sco, action, fields, correlation_id)
        if isinstance(size, Err):
            return fail("VALIDATION.INPUT", f"{where}: the {action} log event cannot be logged: {size.error}",
                        action="Shorten or flatten the part of the request the message names.")
        if size.value > limit:
            return fail("JEV.CONTEXT_EXCEEDED",
                        f"{where}: the {action} log event would be {size.value} bytes, over the run log's {limit}-byte event limit; "
                        "the logger would refuse it after Jev answered",
                        action="Ask fewer questions or options in one request, or shorten the item; never raise a cap to make it fit.")
    return Ok(None)


def worst_answers(questions: dict, pick=None) -> dict:
    """The largest answers Jev can give to ``questions`` that validate_response accepts: every question answered, the choice
    ``pick(options, criteria)`` (default the longest option name) with nearly all the probability and every other option with a
    probability of the longest repr."""
    out = {}
    for qid, question in questions.items():
        criteria = question.get("criteria") if isinstance(question, dict) else None
        options = list(criteria) if isinstance(criteria, dict) and criteria else ["x"]
        choice = pick(options, criteria if isinstance(criteria, dict) else {}) if pick else max(options, key=lambda o: len(json.dumps(o)))
        out[qid] = {"type": "choice", "choice": choice, "confidence": WORST_PROBABILITY,
                    "probabilities": {o: 0.9999999999999999 if o == choice else WORST_PROBABILITY for o in options}}
    return out


def worst_line(job: BatchJob, settings: BatchSettings, pick=None) -> dict:
    """The largest result line ``job`` can produce: the largest answers, a first attempt that failed with the longest
    message and a second that answered (only a transient failure is retried, and its code is short). Messages are clipped
    by characters but the logger measures ASCII-escaped JSON, so the worst message is ATTEMPT_MESSAGE_LIMIT characters that
    each escape to 12 bytes (WORST_CHAR), not "x"s."""
    raw = {"model": "m" * 64, "answers": worst_answers(job.questions, pick), "usage": {"input_tokens": 2 ** 63 - 1, "output_tokens": 2 ** 63 - 1}}
    failed = {"attempt": 1, "ok": False, "error": {"code": "JEV.RESPONSE_INVALID", "message": WORST_CHAR * ATTEMPT_MESSAGE_LIMIT, "recoverable": True}}
    return _line(job, settings, raw, [failed, {"attempt": 2, "ok": True}], None, False)


def check_log_events(job: BatchJob, settings: BatchSettings, where: str, fields: dict) -> Result[None, JudgeError]:
    """REQ-JEV-0211: the request event, the largest possible answer event and the result event of ``job`` fit the logger's
    event limit, before anything is sent (and in a dry run alike)."""
    line = worst_line(job, settings)
    return check_event_sizes([(action, event_fields, line["run_id"]) for action, event_fields in result_line_events(line, fields)], where)


def pump_batch(lines: list, settings: BatchSettings, api_key: str, fixture, log: RunLog, echo: bool,
               fields=None) -> Result[list, JudgeError]:
    """Run the low layer over validated request lines; each result line goes, unchanged, to stdout (when ``echo``)
    and into the run log as its request, answer and result events, flushed after every line so a crash keeps the
    evidence."""

    async def drive() -> Result[list, JudgeError]:
        out = []
        async for line in stream_batch(lines, settings, api_key, fixture):
            logged = log_result_line(log, line, fields or {})
            if isinstance(logged, Err):
                return logged
            if echo:
                write_stdout_line(encode_result_line(line))
            out.append(line)
        return Ok(out)

    try:
        return asyncio.run(drive())
    except BatchInputError as exc:
        return Err(exc.error)


def validate_request_lines(lines: list, settings: BatchSettings) -> Result[int, JudgeError]:
    """Every line checked before any call: a bad line late in the stream never leaves earlier requests half-sent."""
    base = load_question_set(settings.gate)
    if isinstance(base, Err):
        return base
    total, keys = 0, set()
    for number, line in enumerate(lines, start=1):
        jobs = batch_jobs(line, number, settings, base.value.roles)
        if isinstance(jobs, Err):
            return jobs
        for job in jobs.value:
            if job.request_id in keys:
                return fail("VALIDATION.INPUT", f"request {job.request_id} appears twice in the stream",
                            action="Pair each item, anchor and candidate once; use --repeat for repeats.")
            keys.add(job.request_id)
        total += len(jobs.value)
    return Ok(total)


def run_low_layer(args, lines: list) -> Result[Any, JudgeError]:
    """The shared CLI path for `batch` and `bead-batch`: validate, consent, stream, log. Stdout carries only result lines."""
    settings = batch_settings(args, args.request_prefix)
    if isinstance(settings, Err):
        return settings
    settings = settings.value
    checked = validate_request_lines(lines, settings)
    if isinstance(checked, Err):
        return checked
    loaded = load_answers_fixture(args.answers_file, "'<item>/<candidate>/r<n>', '<item>/<candidate>', '<candidate>'")
    if isinstance(loaded, Err):
        return loaded
    fixture = loaded.value
    covered = check_fixture_coverage(lines, settings, fixture)
    if isinstance(covered, Err):
        return covered
    run_dir = args.run_dir or (str(pathlib.Path(args.anchors).parent / "anchor-runs") if getattr(args, "anchors", "") else "")
    if not run_dir:
        return fail("VALIDATION.INPUT", "the result log needs a directory: pass --anchors (log under its anchor-runs/) or --run-dir",
                    action="Pass --anchors <plan>/anchors.json so the log lands in <plan>/anchor-runs/.")
    run_name = args.run_name or stamp()
    checked = _safe_id(run_name, "--run-name")  # refused before the logger creates anything
    if isinstance(checked, Err):
        return checked
    api_key = ""
    if not settings.dry_run and fixture is None:
        cleared = remote_preflight(args, "The anchor statements, scopes and item texts in the request stream")
        if isinstance(cleared, Err):
            return cleared
        api_key = cleared.value
    log = open_run_log(run_dir, run_name)
    if isinstance(log, Err):
        return log
    pumped = pump_batch(lines, settings, api_key, fixture, log.value, echo=True)
    closed = log.value.close()
    if isinstance(pumped, Err):
        return pumped
    if isinstance(closed, Err):
        return closed
    return Ok(StreamedResult([x for x in pumped.value if not x["ok"] and not x["dry_run"]]))


class StreamedResult(NamedTuple):
    """A command whose stdout was the result lines: main prints nothing more; exit 2 when any line has no answer."""
    failed: list


def cmd_batch(args) -> Result[Any, JudgeError]:
    lines = read_request_lines(args.input)
    if isinstance(lines, Err):
        return lines
    return run_low_layer(args, lines.value)


def add_stream_options(parser) -> None:
    """Options shared by `batch` and `bead-batch`: both run the same low layer."""
    parser.add_argument("--run-dir", default="", help="log root of the sc-observability run log (<run-dir>/logs/judge-jev.log.jsonl, appended to); default <anchors dir>/anchor-runs")
    parser.add_argument("--run-name", default="", help="run name carried by every log event; default a UTC timestamp")
    parser.add_argument("--repeat", type=int, default=1, help="identical requests per (item, candidate); default 1")
    parser.add_argument("--request-prefix", default="batch", help="request ids are <prefix>/<item>/<anchor>/<candidate>/r<n>")
    parser.add_argument("--threshold", type=float, default=DEFAULT_THRESHOLD, help="confidence threshold of each line's verdict")
    parser.add_argument("--max-workers", type=int, default=BATCH_MAX_WORKERS, help=f"parallel Jev calls, 1 to {BATCH_MAX_WORKERS}")
    parser.add_argument("--max-request-bytes", type=int, default=None, help=f"refuse any request over this before sending; at most {MAX_REQUEST_BYTES_CEILING} (requests are logged whole); default the gate's cap")
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--dry-run", action="store_true", help="emit the result lines with the requests only; call nothing")
    parser.add_argument("--answers-file", default="", help="fixture answers keyed '<item>/<candidate>/r<n>', '<item>/<candidate>', '<candidate>' or '*'; no network")
    parser.add_argument("--repository-root", default=".", help=REPO_ROOT_HELP)


# --------------------------------------------------------------------------- #
# quality (REQ-JEV-0101, REQ-JEV-0209): records, each kind's decision tree, one batch request per wave
# --------------------------------------------------------------------------- #
QUALITY_TABLES = {"req": "requirements", "nfr": "requirements", "adr": "decisions"}  # Raptor import index tables
QUALITY_PREFIX = "quality"
QUALITY_LOG_DIR = ".sc/judge-jev"  # the logger writes <log root>/logs/judge-jev.log.jsonl


class QualityTree(NamedTuple):
    id: str
    sha256: str          # of the file's bytes, CRLF as LF: variations of one tree are compared by digest
    path: str
    override: bool
    start: str
    nodes: dict          # node id -> Wave or Leaf
    kind: Optional[str]  # the tree's record kind, when it names one
    rule_ranks: Optional[dict] = None  # TTL tree: {wave id: [jj:rank of each rule, in list order]}; JSON: rank = position

    def rule_rank(self, node_id: str, position: int) -> int:
        """The rank of the rule at 1-based ``position`` in wave ``node_id``'s rules: the tree's own ``jj:rank`` for a
        TTL tree (unique, not necessarily consecutive), the position for a JSON tree, whose list order is its rank."""
        ranks = (self.rule_ranks or {}).get(node_id)
        return ranks[position - 1] if ranks else position


def _tree_error(path: str, message: str) -> Err[JudgeError]:
    return fail("QUALITY.TREE", f"quality tree {path}: {message}",
                action="Fix the tree file; see assets/questions/quality-req.json for the shape.")


def parse_quality_tree(raw: str, path: str, override: bool = False) -> Result[QualityTree, JudgeError]:
    """The text of a tree file as a QualityTree: strict JSON, then the QualityTreeModel (every node, question and rule, the
    graph and its cycles). A node is a wave or a leaf ({verdict}). A wave has questions and exactly one of: ``next``, an
    ordered list of {when: {question id: choice}, goto: node} (every pair must match; the first matching rule wins), or
    ``verdict_rules``, an ordered list of {any: {question id: choice}, all: {question id: choice}, verdict} (a rule matches
    when any ``any`` question got its listed choice and every ``all`` question got its listed choice; an absent ``any`` or
    ``all`` holds; the first matching rule gives the verdict and the walk ends at that wave). A listed choice may be a list:
    any of its choices matches.

    A wave with ``per: statement`` is sent once per statement of the record: ``statements`` names the record field
    (dotted path) holding the statements and ``modals``, if given, the statement modals judged. Its rules match when
    any one statement's answers match them."""
    data = decode_json(raw, path, f"quality tree {path}")
    if isinstance(data, Err):
        return data
    parsed = validate_model(QualityTreeModel, data.value)
    if isinstance(parsed, Err):
        return _tree_error(path, parsed.error.text)
    tree = parsed.value
    return Ok(QualityTree(tree.id, content_sha256(raw), path, override, tree.start, tree.nodes, tree.kind))


def parse_quality_tree_ttl(raw: str, path: str, override: bool = False) -> Result[QualityTree, JudgeError]:
    """A tree file in Turtle: loaded and SHACL-validated by jev_graph (its errors as they are), compiled to the quality JSON,
    then parsed as a JSON tree. The digest is that of the file's bytes."""
    import jev_graph

    loaded = jev_graph.load_tree(path)
    if isinstance(loaded, Err):
        return loaded
    checked = jev_graph.validate_tree(loaded.value)
    if isinstance(checked, Err):
        return checked
    parsed = parse_quality_tree(json.dumps(jev_graph.tree_to_quality_json(loaded.value)), path, override)
    if isinstance(parsed, Err):
        return parsed
    return Ok(parsed.value._replace(sha256=content_sha256(raw), rule_ranks=jev_graph.rule_ranks(loaded.value)))


def parse_quality_tree_file(raw: str, path: str, override: bool = False) -> Result[QualityTree, JudgeError]:
    """``parse_quality_tree`` for a .json tree, ``parse_quality_tree_ttl`` for a .ttl tree (by the path's extension)."""
    if path.lower().endswith(".ttl"):
        return parse_quality_tree_ttl(raw, path, override)
    return parse_quality_tree(raw, path, override)


def load_quality_tree(kind: str, override_path: str = "") -> Result[QualityTree, JudgeError]:
    """The record kind's shipped tree, or the --questions override (.json or .ttl; see parse_quality_tree)."""
    path = override_path or str(QUESTIONS_DIR / f"quality-{kind}.json")
    raw = read_source(path, "quality tree", action="Pass the path of an existing tree file.")
    if isinstance(raw, Err):
        return raw
    return parse_quality_tree_file(raw.value, path, bool(override_path))


def _got(choices: dict, qid: str, wanted) -> bool:
    return choices.get(qid) in wanted if isinstance(wanted, list) else choices.get(qid) == wanted


def branch_matches(rule: NextRule, choices: dict) -> bool:
    return all(_got(choices, q, c) for q, c in rule.when.items())


def verdict_matches(rule: VerdictRule, choices: dict) -> bool:
    anyof, allof = rule.any_, rule.all_
    return (not anyof or any(_got(choices, q, c) for q, c in anyof.items())) and all(_got(choices, q, c) for q, c in allof.items())


def select_rule(rules: list, matches, per_choices: list):
    """The first rule that matches the choices of any one request of the wave (one, or one per statement), with the
    indexes (into ``per_choices``) of the requests it matched; (None, []) when none does."""
    for rule in rules:
        hit = [i for i, choices in enumerate(per_choices) if matches(rule, choices)]
        if hit:
            return rule, hit
    return None, []


def wave_reason(node: Wave, answers: dict) -> list:
    """The wave's reason: every question in tree order with its id, text, Jev's choice, the chosen option's criterion
    (which carries its fix hint) and confidence."""
    out = []
    for qid, question in node.questions.items():
        choice, confidence = answer_choice(answers[qid])
        out.append({"id": qid, "text": question.instructions, "choice": choice,
                    "criterion": question.criteria.get(choice, ""), "confidence": confidence})
    return out


def record_statements(record: dict, node: Wave) -> Result[list, JudgeError]:
    """A per-statement wave's statements: the list at the node's ``statements`` field path in the record, each a string or
    an object with text (and modal), kept when its modal is one of the node's ``modals`` (all when absent). Index is the
    statement's 1-based position in that field."""
    value, field = record, node.statements
    for part in field.split("."):
        value = value.get(part) if isinstance(value, dict) else None
    where = f"{record.get('id')} field {field}"
    if not isinstance(value, list):
        return fail("RECORD.STATEMENTS", f"{where} is not a list of statements",
                    action="Fix the tree's statements field path, or the record.")
    out = []
    for i, item in enumerate(value, 1):
        text, modal = (item, None) if isinstance(item, str) else (item.get("text"), item.get("modal")) if isinstance(item, dict) else (None, None)
        if not isinstance(text, str) or not text.strip():
            return fail("RECORD.STATEMENTS", f"{where} item {i} has no statement text", action="Fix the record, rebuild the index, then rerun.")
        if node.modals is None or modal in node.modals:
            out.append({"index": i, "modal": modal, "text": text})
    if not out:
        return fail("RECORD.STATEMENTS", f"{where} has no statement with modal {node.modals}",
                    action="Add the record's MUST or MUST NOT statements, rebuild the index, then rerun.")
    return Ok(out)


def parse_index(data, index_path: str) -> Result[RaptorIndex, JudgeError]:
    parsed = validate_model(RaptorIndex, data)
    if isinstance(parsed, Err):
        return fail("INPUT.MALFORMED", f"{index_path} is not a Raptor import index: {parsed.error.text}",
                    action="Pass the JSON written by Raptor's scripts/extract.py --output.")
    return parsed


def index_repository(index_path: str) -> Optional[str]:
    """The Raptor repository URN of an import index (``metadata.repository``), or None when it has none or is unreadable."""
    loaded = load_json(index_path, "Raptor import index")
    parsed = parse_index(loaded.value, index_path) if isinstance(loaded, Ok) else None
    repository = (parsed.value.metadata or {}).get("repository") if isinstance(parsed, Ok) else None
    return repository if isinstance(repository, str) and repository else None


def index_record_iri(index_path: str, record_id: str) -> str:
    """The Raptor record IRI ``<repository URN>#<record id>`` that definitions attach to (as ``jev_runs.record_iri``)."""
    return jev_runs.record_iri({"repository": index_repository(index_path), "record_id": record_id})


def _graph_module() -> Result[Any, JudgeError]:
    """``jev_graph`` (imported lazily: it needs pyoxigraph, and pyshacl to validate)."""
    try:
        import jev_graph
    except ImportError as exc:
        return fail("DEPENDENCY.MISSING", f"reading definitions needs pyoxigraph and pyshacl and they did not import: {exc}",
                    action="Run with `uv run --script jev_judge.py ...` (its inline metadata pins them), or "
                           "python3 -m pip install 'pyoxigraph>=0.5,<0.6' 'pyshacl>=0.30,<1'.")
    return Ok(jev_graph)


def load_definitions_file(path) -> Result[tuple, JudgeError]:
    """One definitions file: size, UTF-8 and Turtle (``jev_graph.load_definitions``), the closed SHACL shapes, then the pydantic
    body of every definition. ``Ok((rows, warnings))``: the validated rows (``jev_graph.definition_rows``) and the SHACL
    warnings (a definition with no attachment). Every refusal is typed and names the file, the definition and the rule."""
    module = _graph_module()
    if isinstance(module, Err):
        return module
    graph = module.value
    loaded = graph.load_definitions(path)
    if isinstance(loaded, Err):
        return loaded
    checked = graph.validate_definitions(loaded.value)
    if isinstance(checked, Err):
        return checked
    rows = graph.definition_rows(loaded.value)
    for row in rows:
        body = validate_model(DefinitionBody, {"term": row["term"], "text": row["text"], "source": row["source"],
                                               "attachments": row["attachments"]})
        if isinstance(body, Err):
            return fail("DEFINITIONS.INVALID", f"definitions file {path}: definition {row['iri']}: {body.error.text}",
                        action="Fix the definition; see docs/cli.md (Definitions) for the fields and their limits.")
    return Ok((rows, checked.value))


def load_definition_set(paths: list) -> Result[Any, JudgeError]:
    """Every file in ``paths`` loaded and validated; a ``jev_definitions.DefinitionSet`` of all their definitions. The first
    bad file is the error. The definitions are data of their own: they are never loaded into a tree's graph."""
    loaded = []
    for path in paths:
        got = load_definitions_file(path)
        if isinstance(got, Err):
            return got
        loaded.append((str(path), got.value[0]))
    built, problem = jev_definitions.build_set(loaded)
    if built is None:
        return fail("DEFINITIONS.INVALID", problem, action="Keep each definition IRI in one file.")
    return Ok(built)


def definitions_for_args(args) -> Result[Any, JudgeError]:
    """The definitions a command uses: the files named by ``--definitions`` (repeatable), else every definitions file in
    ``<--repository-root>/.sc/judge-jev/``. ``Ok(None)`` when there are none: the requests are then exactly as before."""
    paths = list(getattr(args, "definitions", None) or []) or jev_definitions.discover(getattr(args, "repository_root", "."))
    return load_definition_set(paths) if paths else Ok(None)


def quality_record(index_path: str, kind: str, record_id: str) -> Result[dict, JudgeError]:
    """One record from a Raptor import index (scripts/extract.py output). A record with import diagnostics is refused."""
    loaded = load_json(index_path, "Raptor import index")
    if isinstance(loaded, Err):
        return loaded
    parsed = parse_index(loaded.value, index_path)
    if isinstance(parsed, Err):
        return parsed
    index = parsed.value
    table = getattr(index, QUALITY_TABLES[kind])
    if table is None:
        return fail("INPUT.MALFORMED", f"{index_path} is not a Raptor import index (needs {QUALITY_TABLES[kind]})",
                    action="Pass the JSON written by Raptor's scripts/extract.py --output.")
    found = [i for i in index.diagnostics.issues if i.get("id") == record_id]
    if found:
        error = JudgeError("RECORD.DIAGNOSTICS", f"{record_id} has {len(found)} Raptor import diagnostic(s); only records imported without diagnostics are judged",
                           action="Fix each diagnostic (its remedy says how), rebuild the index, then rerun.")
        error.data = {"record_id": record_id, "diagnostics": found}
        return Err(error)
    record = next((r for r in table if r.get("id") == record_id), None)
    if record is None:
        return fail("RECORD.NOT_FOUND", f"{record_id} is not in the {QUALITY_TABLES[kind]} of {index_path}",
                    action="Check the id and the --req/--adr/--nfr flag, and that the index is current.")
    return Ok(record)


def wave_line(record: dict, node_id: str, node: Wave, statement=None, definitions=(), location=None) -> dict:
    """One wave request as one batch request line, no anchor. The item is the record (as Raptor's JSON); for one
    statement of a per-statement wave, item ``<record id>.s<index>`` whose text is {statement, record} as JSON.
    ``definitions`` (``jev_definitions.Definition`` objects) go into the state as ``[{term, text}]`` and ``location`` as
    ``location``; both are left out of the line, and so out of the request, when empty (REQ-JEV-0212)."""
    if statement is None:
        item = {"id": record["id"], "title": record.get("title") or "", "text": json.dumps(record, indent=2, ensure_ascii=False)}
    else:
        item = {"id": f"{record['id']}.s{statement['index']}", "title": record.get("title") or "",
                "text": json.dumps({"statement": statement, "record": record}, indent=2, ensure_ascii=False)}
    line = {"item": item}
    if location is not None:
        line["location"] = location
    if definitions:
        line["definitions"] = jev_definitions.state_entries(list(definitions))
    line["questions"] = [{"candidate_id": node_id, "questions": {qid: q.model_dump() for qid, q in node.questions.items()}}]
    return line


def wave_lines(record: dict, node_id: str, node: Wave, definitions=(), location=None) -> Result[tuple, JudgeError]:
    """The wave's request lines and their statements: one line (statements [None]), or one per statement."""
    if node.per != "statement":
        return Ok(([wave_line(record, node_id, node, None, definitions, location)], [None]))
    statements = record_statements(record, node)
    if isinstance(statements, Err):
        return statements
    return Ok(([wave_line(record, node_id, node, s, definitions, location) for s in statements.value], statements.value))


def quality_settings(dry_run: bool) -> BatchSettings:
    return BatchSettings("plan", DEFAULT_MODEL, 1, QUALITY_PREFIX, DEFAULT_THRESHOLD, 30.0, 1, DEFAULT_MAX_REQUEST_BYTES, dry_run)


def _with_refs(failed: Err, refs: dict) -> Err:
    """Every error after a tree loads names the record and the tree."""
    if failed.error.data is None:
        failed.error.data = {}
    failed.error.data.update(refs)
    return failed


def quality_fields(tree: QualityTree, record_id: str, kind: str, definitions=(), record_iri: str = "") -> dict:
    """The fields every log event of a record's walk carries; with definitions, each one's IRI, term and digest (REQ-JEV-0212);
    with ``record_iri``, the record's Raptor IRI (``jev_runs.record_iri``), which the rebuilt run triples use."""
    fields = {"tree_id": tree.id, "tree_sha256": tree.sha256, "record_id": record_id, "record_kind": kind}
    if record_iri:
        fields["record_iri"] = record_iri
    if definitions:
        fields["definitions"] = jev_definitions.log_entries(list(definitions))
    return fields


def _longest(values: list):
    return max(values, key=len) if values else None


def check_wave_events(fields: dict, node_id: str, node: Wave, jobs: list, statements: list,
                      settings: BatchSettings, rank: int) -> Result[None, JudgeError]:
    """REQ-JEV-0211: the statement, wave and verdict events a wave's walk logs, with the largest answers and the longest
    next or verdict, fit the logger's event limit. (The request, answer and result events were sized with the request.)"""
    where = f"wave {node_id}"

    def pick(options, criteria):  # the option the wave and statement events carry most of: its name three times and its criterion
        return max(options, key=lambda o: 3 * len(json.dumps(o)) + len(json.dumps(criteria.get(o, ""))))

    results = [worst_line(job, settings, pick) for job in jobs]
    per_choices = [{qid: ans["choice"] for qid, ans in r["answers"].items()} for r in results]
    nxt = _longest([r.goto for r in node.next]) if node.next is not None else None
    verdict = _longest([r.verdict for r in node.verdict_rules]) if node.verdict_rules is not None else None
    events, _wave = wave_log_events({**fields, "node": node_id}, node_id, node, statements, results, per_choices,
                                    list(range(len(results))), nxt, verdict, rank=rank)
    return check_event_sizes(events, where)


def _prepare_record(args, kind: str, record_id: str, trees: dict, settings: BatchSettings, definitions=None,
                    location=None, extra_fields=None) -> Result[tuple, JudgeError]:
    """Load (once per kind) the tree, the record and its wave-1 jobs, one per statement for a per-statement wave (the size
    check). Every wave's statements, request sizes and log events are checked here, before any call, with the record's
    definitions in every request and log event (``definitions``: a DefinitionSet or None; ``location``: the location
    under evaluation, None for ``quality``).
    Ok((tree, record, refs, [(statement or None, job)], carried definitions))."""
    if kind not in trees:
        trees[kind] = load_quality_tree(kind, args.questions)
    tree = trees[kind]
    if isinstance(tree, Err):
        return tree
    tree = tree.value
    refs = {"record": {"id": record_id, "kind": kind, "source": {"raptor_index": args.index}},
            "tree": {"id": tree.id, "sha256": tree.sha256, "source": "override" if tree.override else "shipped", "path": tree.path}}
    if tree.kind is not None and tree.kind != kind:
        return _with_refs(_tree_error(tree.path, f"is a {tree.kind!r} tree; {record_id} is a {kind!r} record"), refs)
    record = quality_record(args.index, kind, record_id)
    if isinstance(record, Err):
        return _with_refs(record, refs)
    carried = definitions.for_request(index_record_iri(args.index, record_id), location) if definitions is not None else []
    if carried:
        refs["definitions"] = jev_definitions.log_entries(carried)
    if location is not None:
        refs["location"] = location
    fields = {**quality_fields(tree, record_id, kind, carried, index_record_iri(args.index, record_id)), **(extra_fields or {})}
    start_jobs = None
    for node_id, node in tree.nodes.items():  # the start wave first (its jobs are returned), then every later wave
        if isinstance(node, Leaf):
            continue
        lines = wave_lines(record.value, node_id, node, carried, location)
        if isinstance(lines, Err):
            return _with_refs(lines, refs)
        wave_jobs = []
        for line, statement in zip(*lines.value):
            first = batch_jobs(line, 1, settings, {}, {**fields, "node": node_id})  # the size check: JEV.CONTEXT_EXCEEDED
            if isinstance(first, Err):
                return _with_refs(first, refs)
            wave_jobs.append((statement, first.value[0]))
        last = len(node.next or node.verdict_rules)  # rules are in rank order: the last has the largest rank
        checked = check_wave_events(fields, node_id, node, [j for _s, j in wave_jobs], lines.value[1], settings,
                                    tree.rule_rank(node_id, last))
        if isinstance(checked, Err):
            return _with_refs(checked, refs)
        if node_id == tree.start:
            start_jobs = wave_jobs
    checked = check_event_sizes([("verdict", {**fields, "verdict": _longest([n.verdict for n in tree.nodes.values() if isinstance(n, Leaf)]
                                                                           + [r.verdict for n in tree.nodes.values() if isinstance(n, Wave)
                                                                              for r in (n.verdict_rules or [])]),
                                              "path": list(tree.nodes)}, None)], "the verdict")
    if isinstance(checked, Err):
        return _with_refs(checked, refs)
    return Ok((tree, record.value, refs, start_jobs, carried))


def dry_run_wave(tree: QualityTree, jobs: list, settings: BatchSettings) -> dict:
    """The composed start-wave request(s) of a dry run, with their sizes (one per statement for a per-statement wave)."""
    if tree.nodes[tree.start].per == "statement":
        wave = {"node": tree.start, "per": "statement",
                "requests": [{"statement": s, "request": job.body, "request_bytes": len(encode_body(job.body))} for s, job in jobs]}
    else:
        (_s, job), = jobs
        wave = {"node": tree.start, "request": job.body, "request_bytes": len(encode_body(job.body))}
    return {**wave, "max_request_bytes": settings.max_request_bytes}


def cmd_quality(args) -> Result[Any, JudgeError]:
    """Every record named by a repeated --req/--adr/--nfr (REQs, then ADRs, then NFRs, each in the order given), under one
    run name and one run log. Every record is
    loaded and size-checked before the first call; a wave without an answer stops the run with the results so far."""
    wanted = [(kind, rid) for kind in ("req", "adr", "nfr") for rid in (getattr(args, kind) or [])]
    if not wanted or any(not rid for _, rid in wanted):
        return fail("VALIDATION.INPUT", "a record id is missing or empty: pass one or more non-empty --req, --adr or --nfr ids")
    settings = quality_settings(args.dry_run)
    loaded_definitions = definitions_for_args(args)
    if isinstance(loaded_definitions, Err):
        return loaded_definitions
    trees, prepared = {}, []
    for kind, rid in wanted:
        got = _prepare_record(args, kind, rid, trees, settings, loaded_definitions.value)
        if isinstance(got, Err):
            return got
        prepared.append(got.value)
    if settings.dry_run:
        results = [{**refs, "wave": dry_run_wave(tree, jobs, settings)} for tree, _record, refs, jobs, _carried in prepared]
        return Ok({"success": True, "error": None, "data": {"dry_run": True, "results": results}})
    run_refs = prepared[0][2] if len(prepared) == 1 else {"records": [refs for _t, _r, refs, _j, _c in prepared]}
    loaded = load_answers_fixture(args.answers_file, "'<record>/<node>/r1', '<record>/<node>', '<node>'")
    if isinstance(loaded, Err):
        return _with_refs(loaded, run_refs)
    fixture = loaded.value
    run_name = args.run_name or stamp()
    checked = _safe_id(run_name, "--run-name")
    if isinstance(checked, Err):
        return _with_refs(checked, run_refs)
    api_key = ""
    if fixture is None:
        cleared = remote_preflight(args, "The record text")
        if isinstance(cleared, Err):
            return _with_refs(cleared, run_refs)
        api_key = cleared.value
    log = open_run_log(args.run_dir, run_name)
    if isinstance(log, Err):
        return _with_refs(log, run_refs)
    results, failed = [], None
    for tree, record, refs, _job, carried in prepared:
        walked = walk_tree(tree, record, settings, api_key, fixture, log.value,
                           quality_fields(tree, record["id"], refs["record"]["kind"], carried,
                                         index_record_iri(args.index, record["id"])), carried)
        if isinstance(walked, Err):
            failed = _with_refs(walked, {**refs, "results": results})
            break
        verdict, path, waves = walked.value
        results.append({**refs, "verdict": verdict, "path": path, "waves": waves})
    closed = log.value.close()
    if failed is not None:
        return failed
    if isinstance(closed, Err):  # the answers were paid for: report them with the log error
        return _with_refs(closed, {**run_refs, "results": results})
    return Ok({"success": True, "error": None, "data": {
        "dry_run": False, "results": results, "log": {"run": run_name, "log_root": args.run_dir}}})


def wave_log_events(fields: dict, node_id: str, node: Wave, statements: list, results: list, per_choices: list,
                    matched: list, nxt, verdict, rank=None) -> tuple:
    """The events a wave logs, as (action, fields, correlation_id), and the wave's entry of the command's result.
    A per-statement wave logs one statement event per statement (its request is the request event with the same
    request_id), then a compact wave event: a single event holding every statement exceeds the logger's event size limit."""
    rank_field = {} if rank is None else {"rule_rank": rank}  # the 1-based rank of the rule that fired (jj:rank in the tree)
    if node.per == "statement":
        reason, events = [], []
        for s, r, c in zip(statements, results, per_choices):
            reason.append({"statement": s, "answers": wave_reason(node, r["answers"])})
            events.append(("statement", {**fields, "node": node_id, "statement": s, "request_id": r["request_id"],
                                         "answers": r["answers"], "choices": c, "reason": reason[-1]["answers"]}, r["run_id"]))
        matched_statements = [statements[i]["index"] for i in matched]
        request_ids = [r["request_id"] for r in results]
        events.append(("wave", {**fields, "node": node_id, "per": "statement", "request_ids": request_ids,
                                "matched_statements": matched_statements, "next": nxt, "verdict": verdict, **rank_field}, None))
        wave = {"node": node_id, "per": "statement", "request_ids": request_ids, "reason": reason,
                "matched_statements": matched_statements, "next": nxt, "verdict": verdict}
        return events, wave
    result, choices = results[0], per_choices[0]
    reason = wave_reason(node, result["answers"])
    events = [("wave", {**fields, "node": node_id, "request_id": result["request_id"], "request": result["request"],
                        "answers": result["answers"], "choices": choices, "reason": reason, "next": nxt,
                        "verdict": verdict, **rank_field}, result["run_id"])]
    return events, {"node": node_id, "request_id": result["request_id"], "reason": reason, "next": nxt, "verdict": verdict}


def _position(rules: list, rule) -> int:
    """The 1-based position of ``rule`` in ``rules``, by identity: two rules with equal bodies are still two rules."""
    return next(i for i, r in enumerate(rules, 1) if r is rule)


def walk_tree(tree: QualityTree, record: dict, settings: BatchSettings, api_key: str, fixture, log: RunLog,
              fields: dict, definitions=(), location=None) -> Result[tuple, JudgeError]:
    """Send one wave per node from the start until a leaf or a wave with verdict_rules; answers pick each next node.
    Every wave reports its reason (each question with Jev's choice and confidence) and is logged with its composed
    request, answers and selected branch or verdict; the verdict is logged with the path."""
    node_id, path, waves = tree.start, [], []
    verdict, decided_rank = None, None
    while verdict is None:
        node = tree.nodes[node_id]
        if isinstance(node, Leaf):
            path.append(node_id)
            verdict = node.verdict
            break
        if node_id in path:
            return _tree_error(tree.path, f"the answers lead back to {node_id}: the tree has a cycle")
        path.append(node_id)
        lines = wave_lines(record, node_id, node, definitions, location)
        if isinstance(lines, Err):
            return lines
        lines, statements = lines.value
        covered = check_fixture_coverage(lines, settings, fixture)  # this wave's request ids are known: refuse before sending
        if isinstance(covered, Err):
            return covered
        pumped = pump_batch(lines, settings, api_key, fixture, log, echo=False, fields={**fields, "node": node_id})
        if isinstance(pumped, Err):
            return pumped
        for result, statement in zip(pumped.value, statements):
            if not result["ok"]:
                which = "" if statement is None else f" statement {statement['index']} ({statement['text']!r})"
                error = JudgeError(result["error"]["code"], f"wave {node_id}{which} got no answer: {result['error']['message']}",
                                   recoverable=result["error"]["recoverable"], action="Rerun once the cause is fixed.")
                error.data = {"path": path, "waves": waves, **({} if statement is None else {"statement": statement})}
                return Err(error)
        per_choices = [{qid: answer_choice(ans)[0] for qid, ans in r["answers"].items()} for r in pumped.value]
        nxt, matched = None, []
        if node.next is not None:
            rule, matched = select_rule(node.next, branch_matches, per_choices)
            if rule is None:
                return _tree_error(tree.path, f"no branch of wave {node_id} matches the answers {per_choices}")
            nxt, rank = rule.goto, tree.rule_rank(node_id, _position(node.next, rule))
        else:
            rule, matched = select_rule(node.verdict_rules, verdict_matches, per_choices)
            if rule is None:
                return _tree_error(tree.path, f"no verdict rule of wave {node_id} matches the answers {per_choices}")
            verdict, rank = rule.verdict, tree.rule_rank(node_id, _position(node.verdict_rules, rule))
            decided_rank = rank
        events, wave = wave_log_events(fields, node_id, node, statements, pumped.value, per_choices, matched, nxt, verdict, rank)
        for action, event_fields, correlation_id in events:
            logged = log.event(action, event_fields, correlation_id=correlation_id)
            if isinstance(logged, Err):
                return logged
        waves.append(wave)
        node_id = nxt
    logged = log.event("verdict", {**fields, "verdict": verdict, "path": path,
                                   **({} if decided_rank is None else {"rule_rank": decided_rank})})
    if isinstance(logged, Err):
        return logged
    flushed = log.flush()
    if isinstance(flushed, Err):
        return flushed
    return Ok((verdict, path, waves))


# --------------------------------------------------------------------------- #
# validate (REQ-JEV-0210): the models and checks the other commands apply before their first Jev request, nothing else
# --------------------------------------------------------------------------- #
VALIDATE_KINDS = (("tree", "quality tree"), ("questions", "question set"), ("answers", "answers file"), ("index", "Raptor import index"),
                  ("requests", "request stream"))


def validate_input(kind: str, raw: str, path: str, args) -> Result[None, JudgeError]:
    """``raw`` (the text of an input of ``kind``) accepted or refused exactly as the command that reads it would."""
    what = dict(VALIDATE_KINDS)[kind]
    if kind == "tree":
        parsed = parse_quality_tree_file(raw, path)
        return Ok(None) if isinstance(parsed, Ok) else parsed
    if kind == "requests":
        lines = parse_request_lines(raw, path)
        if isinstance(lines, Err):
            return lines
        cap = request_cap(args.gate, args.max_request_bytes)
        if isinstance(cap, Err):
            return cap
        settings = BatchSettings(args.gate, DEFAULT_MODEL, 1, "batch", DEFAULT_THRESHOLD, 30.0, 1, cap.value, True)
        checked = validate_request_lines(lines.value, settings)
        return checked if isinstance(checked, Err) else Ok(None)
    data = decode_json(raw, path, what)
    if isinstance(data, Err):
        return data
    if kind == "questions":
        parsed = parse_question_set(data.value, f"question set {path}")
    elif kind == "index":
        parsed = parse_index(data.value, path)
    else:
        enveloped = answers_envelope_error(data.value, f"answers file {path}")
        return Err(enveloped) if enveloped is not None else Ok(None)
    return Ok(None) if isinstance(parsed, Ok) else parsed


def cmd_validate(args) -> Result[Any, JudgeError]:
    """Exactly one input file (``-`` is stdin) checked with its model; never calls Jev, reads no key, writes no log."""
    kind, path = next((k, getattr(args, k)) for k, _what in VALIDATE_KINDS if getattr(args, k) is not None)
    raw = read_source(path, dict(VALIDATE_KINDS)[kind])
    if isinstance(raw, Err):
        return raw
    checked = validate_input(kind, raw.value, path, args)
    if isinstance(checked, Err):
        return checked
    return Ok({"success": True, "error": None, "data": {"kind": kind, "path": path, "sha256": content_sha256(raw.value)}})


# --------------------------------------------------------------------------- #
# compare-definition (REQ-JEV-0213, ADR-JEV-0008): keep definitions only when, together, they change an answer
# --------------------------------------------------------------------------- #
COMPARE_ARMS = ("without", "with")
COMPARE_FIXTURE_KEYS = "'with/<record>/<node>', 'without/<record>/<node>', or unprefixed '<record>/<node>', '<node>', '*' for both arms"


def arm_fixture(fixture, arm: str):
    """The answers fixture one arm reads: keys prefixed ``<arm>/`` are that arm's alone and win; unprefixed keys serve both
    arms (so a fixture of only unprefixed keys gives identical answers, hence not material). ``None`` stays ``None``."""
    if fixture is None:
        return None
    own = {k[len(arm) + 1:]: v for k, v in fixture.items() if k.startswith(arm + "/")}
    shared = {k: v for k, v in fixture.items() if not any(k.startswith(a + "/") for a in COMPARE_ARMS)}
    return {**shared, **own}


def answer_table(waves: list) -> dict:
    """``{(node, statement index or None, question id): (choice, confidence)}`` of a walk's waves."""
    table = {}
    for wave in waves:
        if wave.get("per") == "statement":
            for item in wave["reason"]:
                for a in item["answers"]:
                    table[(wave["node"], item["statement"]["index"], a["id"])] = (a["choice"], a["confidence"])
        else:
            for a in wave["reason"]:
                table[(wave["node"], None, a["id"])] = (a["choice"], a["confidence"])
    return table


def compare_answers(without: dict, with_: dict, threshold: float) -> list:
    """One entry per answer either arm gave, in the order the arms asked them: each side's choice and confidence (None when that
    arm never asked the question, because its answers led down another branch), whether the choice changed (an answer only one
    arm has counts as changed), the confidence change (with minus without, rounded to 9 places), whether the confidence
    crossed ``threshold`` (one side at or above it, the other below) and whether the answer is material: its choice changed
    or its confidence crossed the threshold. A move on the same side of the threshold (0.90 to 0.95) is not material."""
    out = []
    for key in list(without) + [k for k in with_ if k not in without]:
        a, b = without.get(key), with_.get(key)
        changed = a is None or b is None or a[0] != b[0]
        delta = None if a is None or b is None else round(b[1] - a[1], 9)
        crossed = a is not None and b is not None and (a[1] >= threshold) != (b[1] >= threshold)
        out.append({"node": key[0], "statement": key[1], "question": key[2],
                    "without": None if a is None else {"choice": a[0], "confidence": a[1]},
                    "with": None if b is None else {"choice": b[0], "confidence": b[1]},
                    "choice_changed": changed, "confidence_change": delta, "crosses_threshold": crossed,
                    "material": changed or crossed})
    return out


def compare_event_fields(record_id: str, definitions: list, location, threshold: float, answers: list, runs: dict) -> dict:
    """The fields of the ``compare`` log event: the summary, not the answers (each arm's own events hold those)."""
    fields = {"record_id": record_id, "definitions": [d.reference() for d in definitions], "threshold": threshold,
              "material": any(a["material"] for a in answers), "answers_compared": len(answers),
              "answers_material": sum(1 for a in answers if a["material"]),
              "max_confidence_change": max([abs(a["confidence_change"]) for a in answers if a["confidence_change"] is not None] or [0.0]),
              "runs": runs}
    if location is not None:
        fields["location"] = location
    return fields


def cmd_compare_definition(args) -> Result[Any, JudgeError]:
    """Run the requests one or more candidate definitions reach without them and with them, and report each answer's choice and
    confidence on both sides. The candidates are named by ``--definition`` (IRIs, repeatable; from ``--definitions`` or the
    repository's files) and are compared together: the ``without`` arm drops all of them, the ``with`` arm keeps all of them,
    and each must be attached to the record (and to ``--location`` when given). The record's other definitions are in both
    arms. Each arm walks the record's tree and is logged under its own run name ``<run>.without`` and ``<run>.with``; the
    summary is a ``compare`` event under ``<run>``. The candidates are material when an answer's choice flips or its
    confidence crosses ``--threshold`` (REQ-JEV-0213); the threshold applied is in the output and the log."""
    threshold = check_threshold(args.threshold)
    if isinstance(threshold, Err):
        return threshold
    threshold = threshold.value
    wanted = [(kind, rid) for kind in ("req", "adr", "nfr") for rid in (getattr(args, kind) or [])]
    if len(wanted) != 1 or not wanted[0][1]:
        return fail("VALIDATION.INPUT", "pass exactly one non-empty --req, --adr or --nfr id to compare definitions on")
    kind, rid = wanted[0]
    location = None
    if args.location is not None:
        parsed = validate_model(LocationBody, args.location)
        if isinstance(parsed, Err):
            return fail("VALIDATION.INPUT", f"--location: {parsed.error.text}", action="Pass the location as path, path#Heading or path::Symbol, relative to the repository.")
        location = parsed.value.root
    loaded = definitions_for_args(args)
    if isinstance(loaded, Err):
        return loaded
    candidates = []
    for iri in dict.fromkeys(args.definition):
        found = loaded.value.by_iri(iri) if loaded.value is not None else None
        if found is None:
            return fail("DEFINITIONS.NOT_FOUND", f"{iri!r} is not a definition of "
                        + (", ".join(loaded.value.files) if loaded.value is not None else "any definitions file (there is none)"),
                        action="Pass --definition <the definition's IRI> and --definitions <file> (or add a definitions file under .sc/judge-jev/).")
        candidates.append(found)
    drop = {c.iri for c in candidates}
    with_set = loaded.value
    without_set = jev_definitions.DefinitionSet(tuple(d for d in with_set.definitions if d.iri not in drop), with_set.files)
    settings = quality_settings(args.dry_run)
    trees, prepared = {}, {}
    for arm, arm_set in (("with", with_set), ("without", without_set)):
        got = _prepare_record(args, kind, rid, trees, settings, arm_set, location, {"arm": arm})
        if isinstance(got, Err):
            return got
        prepared[arm] = got.value
    carried = {d.iri for d in prepared["with"][4]}
    missing = [c.iri for c in candidates if c.iri not in carried]
    if missing:
        where = f" at location {location!r}" if location is not None else " (record only: pass --location for a definition attached to a record and a location)"
        return _with_refs(fail("DEFINITIONS.NOT_ATTACHED", f"definition(s) {', '.join(missing)} not attached to {rid}{where}; "
                               "a comparison runs only the requests the definitions would reach",
                               action="Attach each definition to the record (jj:attachedTo), or name the location it is attached to."),
                          prepared["with"][2])
    head = {"record": prepared["with"][2]["record"], "tree": prepared["with"][2]["tree"],
            "definitions": [{**c.reference(), "source": c.source} for c in candidates], "threshold": threshold,
            **({} if location is None else {"location": location})}
    if settings.dry_run:
        arms = {arm: {"definitions": jev_definitions.log_entries(prepared[arm][4]), "wave": dry_run_wave(prepared[arm][0], prepared[arm][3], settings)}
                for arm in COMPARE_ARMS}
        return Ok({"success": True, "error": None, "data": {"dry_run": True, **head, "arms": arms}})
    loaded_answers = load_answers_fixture(args.answers_file, COMPARE_FIXTURE_KEYS)
    if isinstance(loaded_answers, Err):
        return _with_refs(loaded_answers, head)
    fixture = loaded_answers.value
    run_name = args.run_name or stamp()
    runs = {arm: f"{run_name}.{arm}" for arm in COMPARE_ARMS}
    for name in (run_name, *runs.values()):
        checked = _safe_id(name, "--run-name")
        if isinstance(checked, Err):
            return _with_refs(checked, head)
    worst = compare_event_fields(rid, candidates, location, threshold,
                                 [{"material": True, "confidence_change": WORST_PROBABILITY}] * 1000, runs)  # the largest summary: counts of 4 digits
    sized = check_event_sizes([("compare", {"tree_id": prepared["with"][0].id, **worst}, None)], "the compare event")
    if isinstance(sized, Err):
        return _with_refs(sized, head)
    api_key = ""
    if fixture is None:
        cleared = remote_preflight(args, "The record text and the definitions")
        if isinstance(cleared, Err):
            return _with_refs(cleared, head)
        api_key = cleared.value
    walks = {}
    for arm in COMPARE_ARMS:
        tree, record, _refs, _jobs, arm_carried = prepared[arm]
        log = open_run_log(args.run_dir, runs[arm])
        if isinstance(log, Err):
            return _with_refs(log, head)
        walked = walk_tree(tree, record, settings, api_key, arm_fixture(fixture, arm), log.value,
                           {**quality_fields(tree, rid, kind, arm_carried, index_record_iri(args.index, rid)), "arm": arm}, arm_carried, location)
        closed = log.value.close()
        if isinstance(walked, Err):
            return _with_refs(walked, {**head, "arm": arm})
        if isinstance(closed, Err):
            return _with_refs(closed, {**head, "arm": arm})
        walks[arm] = walked.value
    answers = compare_answers(answer_table(walks["without"][2]), answer_table(walks["with"][2]), threshold)
    log = open_run_log(args.run_dir, run_name)
    if isinstance(log, Err):
        return _with_refs(log, head)
    logged = log.value.event("compare", {"tree_id": prepared["with"][0].id,
                                         **compare_event_fields(rid, candidates, location, threshold, answers, runs)})
    closed = log.value.close()
    for failed in (logged, closed):
        if isinstance(failed, Err):  # the answers were paid for: report them with the log error
            return _with_refs(failed, {**head, "answers": answers})
    return Ok({"success": True, "error": None, "data": {
        "dry_run": False, **head, "material": any(a["material"] for a in answers),
        "answers_compared": len(answers), "answers_material": sum(1 for a in answers if a["material"]), "answers": answers,
        "arms": {arm: {"verdict": walks[arm][0], "path": walks[arm][1], "run": runs[arm],
                       "definitions": jev_definitions.log_entries(prepared[arm][4])} for arm in COMPARE_ARMS},
        "log": {"run": run_name, "log_root": args.run_dir}}})


# --------------------------------------------------------------------------- #
# doctor (REQ-JEV-0214, REQ-JEV-0215): check everything judge-jev loads in a repository, call nothing
# --------------------------------------------------------------------------- #
def _issue(severity: str, code: str, file, message: str, **extra) -> dict:
    return {"severity": severity, "code": code, "file": None if file is None else str(file), "message": message, **extra}


def _error_issue(file, error, **extra) -> dict:
    data = getattr(error, "data", None)
    violations = data.get("violations") if isinstance(data, dict) else None
    return _issue("error", error.code, file, error.message, **({"violations": violations} if violations else {}), **extra)


def _heading_texts(text: str) -> set:
    return {m.group(1).strip() for m in (re.match(r"^ {0,3}#{1,6}\s+(.*?)\s*#*\s*$", line) for line in text.splitlines()) if m}


def location_problem(root: pathlib.Path, location: str) -> Optional[str]:
    """Why a location string (``path``, ``path#Heading`` or ``path::Symbol``) does not exist in the repository, or None. A
    symbol is not resolved (that needs sc-agent's grammars): only its file."""
    path, kind, detail = jev_definitions.split_location(location)
    try:
        target = (root / path).resolve()
        inside = root.resolve() in target.parents
        is_file = target.is_file()
        headings = _heading_texts(target.read_text(encoding="utf-8", errors="replace")) if kind == "heading" and is_file else set()
    except (OSError, ValueError, RuntimeError):
        return f"{path} cannot be read in the repository"
    if not inside:
        return f"{path} is outside the repository"
    if not is_file:
        return f"{path} does not exist in the repository"
    if kind == "heading" and detail.strip() not in headings:
        return f"{path} has no heading {detail!r}"
    return None


def definition_warnings(file: str, rows: list, root: pathlib.Path, index) -> list:
    """The warnings (never failures) of one definitions file: no attachment, an attachment to a record the Raptor index does not
    hold (``index`` is ``(repository, record ids)`` or None to skip), an attachment to a location that does not exist, a source
    path that does not exist."""
    out = []
    for row in rows:
        who = f"definition {row['iri']} ({row['term']!r})"
        if not row["attachments"]:
            out.append(_issue("warning", "DEFINITION.UNATTACHED", file, f"{who} has no jj:attachedTo: no request carries it", definition=row["iri"]))
        for att in row["attachments"]:
            repository, _hash, record_id = att["record"].partition("#")
            if index is not None and (record_id not in index[1] or (index[0] is not None and repository != index[0])):
                out.append(_issue("warning", "DEFINITION.RECORD_UNKNOWN", file, f"{who} is attached to {att['record']}, which the Raptor index does not hold",
                                  definition=row["iri"], record=att["record"]))
            if att["location"] is not None:
                problem = location_problem(root, att["location"])
                if problem is not None:
                    out.append(_issue("warning", "DEFINITION.LOCATION_MISSING", file, f"{who} is attached to location {att['location']!r}: {problem}",
                                      definition=row["iri"], location=att["location"]))
        if not jev_definitions_source_is_iri(row["source"]):
            path = row["source"].partition("::")[0].partition("#")[0]
            try:
                present = (root / path).is_file()
            except OSError:  # a component too long for the file system is a source that does not exist
                present = False
            if not present:
                out.append(_issue("warning", "DEFINITION.SOURCE_MISSING", file, f"{who} has source {row['source']!r}: {path} does not exist in the repository",
                                  definition=row["iri"], source=row["source"]))
    return out


def jev_definitions_source_is_iri(source: str) -> bool:
    return bool(IRI_TEXT.fullmatch(source))


class DoctorScope:
    """What one doctor run has seen: the files checked, the issues, the questions by id, the definition rows by file."""

    def __init__(self):
        self.checked, self.issues, self.notes = [], [], []
        self.questions: dict = {}   # question IRI -> [tree files declaring it]
        self.definition_rows: list = []  # [(file, rows)]


def _doctor_tree_json(scope: DoctorScope, path: str, raw: str, data) -> None:
    if isinstance(data, dict) and "nodes" in data:
        scope.checked.append({"kind": "quality tree", "path": path})
        parsed = parse_quality_tree(raw, path)
        if isinstance(parsed, Err):
            scope.issues.append(_error_issue(path, parsed.error))
    elif isinstance(data, dict) and "questions" in data:
        scope.checked.append({"kind": "question set", "path": path})
        parsed = parse_question_set(data, f"question set {path}")
        if isinstance(parsed, Err):
            scope.issues.append(_error_issue(path, parsed.error))
    else:
        scope.checked.append({"kind": "unknown", "path": path})
        scope.issues.append(_issue("error", "INPUT.MALFORMED", path, f"{path} is JSON but neither a quality tree (it needs nodes) nor a question set (it needs questions)"))


def _doctor_json(scope: DoctorScope, path: str) -> None:
    raw = read_source(path, f"file {path}")
    if isinstance(raw, Err):
        scope.checked.append({"kind": "unknown", "path": path})
        return scope.issues.append(_error_issue(path, raw.error))
    data = decode_json(raw.value, path, f"file {path}")
    if isinstance(data, Err):
        scope.checked.append({"kind": "unknown", "path": path})
        return scope.issues.append(_error_issue(path, data.error))
    _doctor_tree_json(scope, path, raw.value, data.value)


def _doctor_tree_ttl(scope: DoctorScope, path: str) -> None:
    scope.checked.append({"kind": "quality tree", "path": path})
    module = _graph_module()
    if isinstance(module, Err):
        return scope.issues.append(_error_issue(path, module.error))
    graph = module.value
    loaded = graph.load_tree(path)
    checked = graph.validate_tree(loaded.value) if isinstance(loaded, Ok) else loaded
    if isinstance(checked, Err):
        return scope.issues.append(_error_issue(path, checked.error))
    by_id: dict = {}
    for row in loaded.value.store.query("PREFIX jj: <urn:judge-jev:ontology#> SELECT ?q ?id WHERE { ?q a jj:Question ; jj:id ?id }"):
        scope.questions.setdefault(row["q"].value, []).append(path)
        by_id.setdefault(row["id"].value, set()).add(row["q"].value)
    for qid, iris in sorted(by_id.items()):
        if len(iris) > 1:
            scope.issues.append(_issue("error", "QUESTIONS.DUPLICATE_ID", path, f"{path}: question id {qid!r} is used by {len(iris)} questions of one tree: "
                                       f"{', '.join(sorted(iris))}; a jj:id is unique within its tree", question_id=qid, iris=sorted(iris)))

def _doctor_definitions(scope: DoctorScope, path: str, root: pathlib.Path, index) -> None:
    scope.checked.append({"kind": "definitions", "path": path})
    got = load_definitions_file(path)
    if isinstance(got, Err):
        return scope.issues.append(_error_issue(path, got.error))
    rows = got.value[0]
    scope.definition_rows.append((path, rows))
    scope.issues.extend(definition_warnings(path, rows, root, index))


def _doctor_file(scope: DoctorScope, path: str, root: pathlib.Path, index) -> None:
    """One file, whatever it holds: an expected failure is an issue, and so is an unexpected one (never a traceback)."""
    name = pathlib.Path(path).name
    try:
        if name.endswith(".ttl"):
            if jev_definitions.is_definitions_file(name):
                _doctor_definitions(scope, path, root, index)
            else:
                _doctor_tree_ttl(scope, path)
        elif name.endswith(".json"):
            _doctor_json(scope, path)
        else:
            scope.checked.append({"kind": "unknown", "path": path})
            scope.issues.append(_issue("error", "VALIDATION.INPUT", path, f"{path} is not a .ttl or .json file: doctor checks trees, question sets and definitions files"))
    except Exception as exc:  # noqa: BLE001 - a typed issue beats a traceback; the rest of the scope is still checked
        scope.issues.append(_issue("error", "SCRIPT.INTERNAL", path, f"{type(exc).__name__}: {exc}"))


def doctor_scope_files(args) -> list:
    """The files of the scope, in order, each once: the shipped trees and question sets, the trees and definitions under
    ``<root>/.sc/judge-jev/`` (top level only) and the paths given as arguments (a directory is searched through all
    its subfolders except ``jev_definitions.SKIPPED_DIRS``)."""
    root = pathlib.Path(args.repository_root or ".")
    files = sorted(str(p) for p in QUESTIONS_DIR.iterdir() if p.is_file() and p.suffix in (".json", ".ttl"))
    for found in jev_definitions.scan(root / jev_definitions.DEFINITIONS_DIR, recursive=False):  # as quality's discover: top level only
        files += found
    for given in args.paths:
        p = pathlib.Path(given)
        if p.is_dir():
            for found in jev_definitions.scan(p):
                files += found
        else:
            files.append(given)
    seen, out = set(), []
    for f in files:
        key = os.path.abspath(f)
        if key not in seen:
            seen.add(key)
            out.append(f)
    return out


def cmd_doctor(args) -> Result[Any, JudgeError]:
    """Check the repository scope and call nothing (no Jev, no key, no log): every tree, question set and definitions file
    against its model and shapes (errors, exit 2 with the report in ``data``), the question ids of the scope for
    duplicates (errors), and the warnings of ``definition_warnings`` (exit 0 when nothing else is wrong)."""
    root = pathlib.Path(args.repository_root or ".")
    scope = DoctorScope()
    index = None
    if args.index:
        loaded = load_json(args.index, "Raptor import index")
        parsed = parse_index(loaded.value, args.index) if isinstance(loaded, Ok) else loaded
        if isinstance(parsed, Err):
            scope.checked.append({"kind": "Raptor import index", "path": args.index})
            scope.issues.append(_error_issue(args.index, parsed.error))
        else:
            scope.checked.append({"kind": "Raptor import index", "path": args.index})
            records = {r["id"] for table in (parsed.value.requirements, parsed.value.decisions) for r in (table or [])}
            repository = (parsed.value.metadata or {}).get("repository")
            index = (repository if isinstance(repository, str) and repository else None, records)
    else:
        scope.notes.append({"code": "DOCTOR.INDEX_SKIPPED", "message": "no --index given: attachments were not checked against a Raptor index"})
    for path in doctor_scope_files(args):
        if not pathlib.Path(path).exists():
            scope.issues.append(_issue("error", "INPUT.MISSING", path, f"{path} does not exist"))
            continue
        _doctor_file(scope, path, root, index)
    if index is not None and index[0] is None and any(rows for _f, rows in scope.definition_rows):
        scope.issues.append(_issue("warning", "DEFINITION.REPOSITORY_UNKNOWN", args.index,
                                   "the Raptor index names no metadata.repository, so a record IRI is urn:raptor:repo:unknown#<id> and no attachment "
                                   "of these definitions can match: quality and compare-definition carry none of them"))
    built, problem = jev_definitions.build_set([(f, rows) for f, rows in scope.definition_rows])
    if built is None:
        scope.issues.append(_issue("error", "DEFINITIONS.INVALID", None, problem))
    if any(a["location"] is not None and "::" in a["location"] for _f, rows in scope.definition_rows for row in rows for a in row["attachments"]):
        scope.notes.append({"code": "DOCTOR.SYMBOL_UNCHECKED", "message": "a path::Symbol location was checked for its file only: symbols resolve with sc-agent's grammars"})
    for question, files in sorted(scope.questions.items()):
        if len(files) > 1:  # one IRI declared in several files would merge silently in the store
            scope.issues.append(_issue("error", "QUESTIONS.DUPLICATE_IRI", None,
                                       f"question {question} is declared in {len(files)} tree files: {', '.join(files)}; a question IRI is declared in one file only",
                                       question=question, files=files))
    errors = [i for i in scope.issues if i["severity"] == "error"]
    warnings = [i for i in scope.issues if i["severity"] == "warning"]
    data = {"ok": not errors, "errors": errors, "warnings": warnings, "notes": scope.notes, "checked": scope.checked,
            "counts": {"files": len(scope.checked), "errors": len(errors), "warnings": len(warnings)}}
    if errors:
        first = errors[0]
        error = JudgeError("DOCTOR.FAILED", f"{len(errors)} error(s) and {len(warnings)} warning(s); first: {first['code']}"
                           + (f" in {first['file']}" if first["file"] else "") + f": {_clip(first['message'])}",
                           action="Fix each error in data.errors (each names its file and the rule), then rerun doctor.")
        error.data = data
        return Err(error)
    return Ok({"success": True, "error": None, "data": data})


class _UsageError(Exception):
    pass


class _JsonArgumentParser(argparse.ArgumentParser):
    """Usage errors become a VALIDATION.INPUT envelope (exit 2) instead of argparse's text on stderr."""

    def error(self, message):  # argparse hook; never returns
        raise _UsageError(f"{self.prog}: {message}")


DEFINITIONS_HELP = ("a definitions file (Turtle, ADR-JEV-0008); repeat it for several. Default: every definitions.ttl and "
                    "*.definitions.ttl in <--repository-root>/.sc/judge-jev/ when there are any. A request carries the "
                    "definitions attached to its record (and location); with none attached it is exactly as without definitions")


def build_parser() -> argparse.ArgumentParser:
    p = _JsonArgumentParser(prog="jev_judge.py", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True, parser_class=_JsonArgumentParser)
    bt = sub.add_parser("batch", help="low layer: JSONL request lines in (stdin or --input), one JSONL result line per (item, candidate, repeat) out, logged")
    bt.add_argument("--input", default="-", help="request stream, one JSON object per line (assets/schemas/jev-batch-request.schema.json); - or omitted reads stdin")
    add_stream_options(bt)
    bt.add_argument("--anchors", default="", help="anchors.json; the result log goes to its anchor-runs/ folder unless --run-dir is given")
    bt.add_argument("--gate", choices=("plan", "dev"), default="plan", help="default roles and request cap when a candidate carries no roles")
    bt.set_defaults(func=cmd_batch)
    q = sub.add_parser("quality", help="judge whether each named REQ, ADR or NFR is well written with its kind's decision tree, one Jev request per wave")
    for kind in ("req", "adr", "nfr"):
        q.add_argument(f"--{kind}", metavar="ID", action="append", default=[],
                       help=f"a {kind.upper()} record id; repeat it (and mix kinds) to judge several records under one run name")
    q.add_argument("--index", required=True, help="the Raptor import index (JSON written by Raptor's scripts/extract.py --output)")
    q.add_argument("--questions", default="", help="a decision-tree file (.json or .ttl) overriding the shipped assets/questions/quality-<kind>.json")
    q.add_argument("--dry-run", action="store_true", help="report each record's tree and composed wave-1 request (one per statement for a per-statement wave); call nothing")
    q.add_argument("--answers-file", default="", help="fixture answers keyed '<record>/<node>/r1', '<record>/<node>', '<node>' or '*'; no network")
    q.add_argument("--run-dir", default=QUALITY_LOG_DIR, help=f"log root of the sc-observability run log (<run-dir>/logs/judge-jev.log.jsonl); default {QUALITY_LOG_DIR}")
    q.add_argument("--run-name", default="", help="run name carried by every log event; default a UTC timestamp")
    q.add_argument("--repository-root", default=".", help=REPO_ROOT_HELP)
    q.add_argument("--definitions", metavar="FILE", action="append", default=[], help=DEFINITIONS_HELP)
    q.set_defaults(func=cmd_quality)
    v = sub.add_parser("validate", help="check one input file with the model the other commands apply before their first Jev request; calls nothing, needs no key, writes no log")
    group = v.add_mutually_exclusive_group(required=True)
    group.add_argument("--tree", metavar="FILE", default=None, help="a quality decision tree (assets/questions/quality-<kind>.json or .ttl); - is stdin (JSON)")
    group.add_argument("--questions", metavar="FILE", default=None, help="a question set (assets/questions/plan.json); - is stdin")
    group.add_argument("--answers", metavar="FILE", default=None, help="an answers file (--answers-file); - is stdin")
    group.add_argument("--index", metavar="FILE", default=None, help="a Raptor import index (Raptor's scripts/extract.py --output); - is stdin")
    group.add_argument("--requests", metavar="FILE", default=None, help="a batch request stream (assets/schemas/jev-batch-request.schema.json); - is stdin")
    v.add_argument("--gate", choices=("plan", "dev"), default="plan", help="with --requests: the gate whose roles and request cap apply, as for batch")
    v.add_argument("--max-request-bytes", type=int, default=None, help="with --requests: refuse any request over this, as for batch")
    v.set_defaults(func=cmd_validate)
    cd = sub.add_parser("compare-definition", help="run the requests one or more candidate definitions reach without them and with them; report each answer's choice and confidence on both sides and whether they are material")
    for kind in ("req", "adr", "nfr"):
        cd.add_argument(f"--{kind}", metavar="ID", action="append", default=[], help=f"the {kind.upper()} record id to compare on (exactly one record in all)")
    cd.add_argument("--index", required=True, help="the Raptor import index (JSON written by Raptor's scripts/extract.py --output)")
    cd.add_argument("--definition", required=True, action="append", metavar="IRI", help="the IRI of a candidate definition; repeat it to compare several together. Each must be attached to the record (and to --location when given)")
    cd.add_argument("--definitions", metavar="FILE", action="append", default=[], help=DEFINITIONS_HELP)
    cd.add_argument("--location", default=None, help="the location the candidate is attached to together with the record (path, path#Heading or path::Symbol); omit for a record-only definition")
    cd.add_argument("--threshold", type=float, default=DEFAULT_THRESHOLD, help="the decision threshold: the candidates are material when an answer's choice flips or its confidence crosses it (default %s); recorded in the output and the log" % DEFAULT_THRESHOLD)
    cd.add_argument("--questions", default="", help="a decision-tree file overriding the shipped assets/questions/quality-<kind>.json")
    cd.add_argument("--dry-run", action="store_true", help="report the start-wave request of each arm, without and with the definitions; call nothing")
    cd.add_argument("--answers-file", default="", help="fixture answers keyed " + COMPARE_FIXTURE_KEYS + "; no network")
    cd.add_argument("--run-dir", default=QUALITY_LOG_DIR, help=f"log root of the sc-observability run log; default {QUALITY_LOG_DIR}")
    cd.add_argument("--run-name", default="", help="run name of the compare event; the arms log as <run name>.without and <run name>.with; default a UTC timestamp")
    cd.add_argument("--repository-root", default=".", help=REPO_ROOT_HELP)
    cd.set_defaults(func=cmd_compare_definition)
    dr = sub.add_parser("doctor", help="check every tree, question set and definitions file of the repository scope; calls nothing, needs no key, writes no log")
    dr.add_argument("paths", nargs="*", metavar="PATH", help="more trees, question sets or definitions files, or folders of them, to check besides the shipped ones and those under .sc/judge-jev/")
    dr.add_argument("--index", default="", help="a Raptor import index: attachments to records it does not hold are warnings; without it that check is skipped and doctor says so")
    dr.add_argument("--repository-root", default=".", help="the repository: its .sc/judge-jev/ holds the trees and definitions, and attachment locations and source paths are relative to it (default .)")
    dr.set_defaults(func=cmd_doctor)
    return p


CANCELED_EXIT = 130


def canceled_envelope() -> dict:
    """The interrupt envelope: typed like every error, plus the canceled / aborted_by flags."""
    out = JudgeError("CANCELED", "interrupted before the command finished", recoverable=True,
                     action="Rerun the command; a judge run is written only when it completes, so nothing partial was recorded.").to_json()
    out.update({"canceled": True, "aborted_by": "user"})
    return out


def main(argv=None) -> int:
    """0 when the command ran (its outcome, negative or not, is in data); 2 when it could not; 130 on interrupt."""
    try:
        return _main(argv)
    except KeyboardInterrupt:
        emit(canceled_envelope())
        return CANCELED_EXIT


def _main(argv=None) -> int:
    bad = next((a for a in (sys.argv[1:] if argv is None else argv) if isinstance(a, str) and utf8_text(a) != a), None)
    if bad is not None:
        emit(JudgeError("VALIDATION.INPUT", f"argument {utf8_text(bad)!r} is not valid UTF-8",
                        action="Pass ids, paths and names as UTF-8 text.").to_json())
        return 2
    try:
        args = build_parser().parse_args(argv)
    except _UsageError as exc:
        emit(JudgeError("VALIDATION.INPUT", str(exc), action="Run 'jev_judge.py <subcommand> --help' for the accepted arguments.").to_json())
        return 2
    fmt = getattr(args, "format", "json")
    try:
        outcome = args.func(args)
    except Exception as exc:  # noqa: BLE001 - a JSON error object beats a traceback for the calling agent
        emit(JudgeError("SCRIPT.INTERNAL", f"{type(exc).__name__}: {exc}",
                        action="Report this with the command line; check the JSON inputs against references/anchor-document.md.").to_json())
        return 2
    if isinstance(outcome, Err):
        emit(outcome.error.to_json())
        return 2
    if isinstance(outcome.value, StreamedResult):  # stdout already carried the result lines and nothing else
        failed = outcome.value.failed
        if not failed:
            return 0
        first = failed[0]["error"] or {}
        error = JudgeError(first.get("code") or "EXECUTION.JEV",
                           f"{len(failed)} request(s) got no answer; first {failed[0]['request_id']}: {first.get('message')}",
                           recoverable=bool(first.get("recoverable")),
                           action="Read each failed line's error field; rerun only those requests once the cause is fixed.")
        sys.stderr.write(json.dumps(error.to_json(), sort_keys=True) + "\n")  # stdout stays pure result lines
        return 2
    emit(outcome.value, fmt)
    return 0


if __name__ == "__main__":
    try:
        utf8_stdio()
        code = main()
    except KeyboardInterrupt:  # between the imports and main's own handler
        emit(canceled_envelope())
        code = CANCELED_EXIT
    sys.exit(code)
