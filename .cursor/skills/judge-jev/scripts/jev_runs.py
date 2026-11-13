"""jev_runs.py: a quality run as triples (ADR-JEV-0006 run vocabulary, ADR-JEV-0007 projection).

``run_triples`` projects one record's run (its result lines and walk) onto the ``jj:`` run vocabulary
(``jj: <urn:judge-jev:ontology#>``): ``jj:Run``, ``jj:WaveRun``, ``jj:Answer`` and ``jj:OptionProbability``.
``rebuild_from_log`` makes the same triples from the sc-observability run log alone, which is what makes the log the
record and the triples a projection. ``TRACE_QUERY`` walks a verdict back to the question and option behind it.

IRIs. Every IRI is derived from the run name, the request ids and the tree, never from a random id, so a rebuild is
identical to the original:

* run        ``urn:judge-jev:run:<run name>:<record id>`` (one run name may judge several records)
* wave run   ``urn:judge-jev:waverun:<run name>:<request id>`` (one per request: per statement for a per-statement wave)
* answer     ``urn:judge-jev:answer:<run name>:<request id>:<question id>``; its probability node appends ``:<option>``
* record     ``<repository>#<record id>``, where ``<repository>`` is the Raptor index's ``metadata.repository`` URN
  (``urn:raptor:repo:...``), so a record is ``urn:raptor:repo:<name>#REQ-T-0001``: the repository names the Raptor
  store and the fragment the record, as Raptor's own references do. A run whose log lacks the repository uses
  ``urn:raptor:repo:unknown``; ``run["repository"]`` (or a ``repository`` field the log carries) supplies it.
* tree, node, question, option, rule: for a TreeGraph, the tree file's own IRIs (``jev_graph.iri_index``; no naming
  scheme is assumed). For a quality JSON dict, which has no IRIs, ``JsonIris`` mints the names the shipped trees use
  (the question IRI ``urn:judge-jev:question:<kind>.<id>``).

Standard library at import; ``pyoxigraph`` is imported where triples are built and ``sc_observability`` where the log is read.
"""

from __future__ import annotations

import pathlib
import re
import sys
from typing import Any, Optional
from urllib.parse import quote

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from result import Err, Ok, Result  # noqa: E402

JJ = "urn:judge-jev:ontology#"
RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
XSD_DOUBLE = "http://www.w3.org/2001/XMLSchema#double"
XSD_INTEGER = "http://www.w3.org/2001/XMLSchema#integer"
UNKNOWN_REPOSITORY = "urn:raptor:repo:unknown"
LOG_SERVICE = "judge-jev"
LOG_LIMIT = 1000  # the most events one sc-observability query returns

TRACE_QUERY = """\
PREFIX jj: <urn:judge-jev:ontology#>
SELECT ?run ?record ?verdict ?rule ?question ?instructions ?optionLabel ?criterion ?confidence
WHERE {
  VALUES (?run ?record) { (<{run}> <{record}>) }
  ?run a jj:Run ; jj:record ?record ; jj:verdictValue ?verdict .
  ?waveRun jj:inRun ?run ; jj:fired ?rule .
  ?answer jj:ofWaveRun ?waveRun ; jj:toQuestion ?question ; jj:chose ?option ; jj:confidence ?confidence .
  ?question jj:instructions ?instructions .
  ?option jj:label ?optionLabel ; jj:criterion ?criterion .
}
ORDER BY ?rule ?question
"""
"""Verdict, fired rule, each answer, its question's instructions, the chosen option's label and criterion, for one run
and record. ``{run}`` and ``{record}`` are the run and Raptor record IRIs; ``trace_query`` fills them. The tree's
triples must be in the same store (its question, option and rule resources)."""


def trace_query(run_iri: str, record_iri: str) -> str:
    """``TRACE_QUERY`` for one run and record (IRIs from ``run_iri`` and ``record_iri``)."""
    return TRACE_QUERY.replace("{run}", run_iri).replace("{record}", record_iri)


def slug(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-") or "x"


class JsonIris:
    """The names of a quality tree's resources, derived from the tree's JSON (``id``, ``kind``, ``nodes``).

    The names the shipped TTL trees use: tree ``urn:judge-jev:tree:<tree id>``; node ``<tree IRI>:<node id>``; rule
    ``<node IRI>:rule-<rank>`` with rank the rule's 1-based position; question ``urn:judge-jev:question:<kind>.<question
    id>``; option ``urn:judge-jev:option:<kind>.<question id>:<slug of the label>``. ``kind`` is the tree's kind, or its id.
    """

    def __init__(self, tree: dict):
        self.tree_id = tree["id"]
        self.kind = tree.get("kind") or tree["id"]

    def tree(self) -> str:
        return f"urn:judge-jev:tree:{self.tree_id}"

    def node(self, node_id: str) -> str:
        return f"{self.tree()}:{node_id}"

    def question(self, question_id: str) -> str:
        return f"urn:judge-jev:question:{self.kind}.{question_id}"

    def option(self, question_id: str, label: str) -> str:
        return f"urn:judge-jev:option:{self.kind}.{question_id}:{slug(label)}"

    def rule(self, node_id: str, rank: int) -> str:
        return f"{self.node(node_id)}:rule-{rank}"


def _quality_json(tree: Any) -> dict:
    """The tree as the quality JSON dict: a dict as is, a TreeGraph through ``jev_graph.tree_to_quality_json``."""
    if isinstance(tree, dict):
        return tree
    import jev_graph
    return jev_graph.tree_to_quality_json(tree)


def _digest(tree: Any, run: dict) -> str:
    if isinstance(tree, dict):
        return run["tree_sha256"]
    import jev_graph
    digest = jev_graph.tree_digest(tree)
    if isinstance(digest, Err):
        raise jev_graph.TreeInvalid(digest.error)
    return digest.value


class _JsonNamer(JsonIris):
    """``JsonIris`` with the node-qualified signatures of ``jev_graph.GraphIris``: the JSON names ignore the node."""

    def question(self, node_id: str, question_id: str) -> str:  # type: ignore[override]
        return super().question(question_id)

    def option(self, node_id: str, question_id: str, label: str) -> str:  # type: ignore[override]
        return super().option(question_id, label)


def _iris(tree: Any, quality: dict):
    if isinstance(tree, dict):
        return _JsonNamer(quality)
    import jev_graph
    return jev_graph.iri_index(tree)


def record_iri(run: dict) -> str:
    return f"{run.get('repository') or UNKNOWN_REPOSITORY}#{run['record_id']}"


def run_iri(run: dict) -> str:
    return f"urn:judge-jev:run:{run['name']}:{run['record_id']}"


def _choice(ans: dict) -> tuple:
    """(choice, confidence) of an answer, as jev_judge.answer_choice reads it (a noul answer is yes at p >= 0.5)."""
    noul = ans.get("type") == "noul" or ("type" not in ans and "noul" in ans)
    if noul:
        value = next((ans[k] for k in ("noul", "probability", "p") if k in ans), None)
        p = float(value or 0.0)
        return ("yes" if p >= 0.5 else "no", abs(p - 0.5) * 2)
    return (ans.get("choice", "unclear"), float(ans.get("confidence", 0.0)))


def _got(choices: dict, qid: str, wanted) -> bool:
    return choices.get(qid) in wanted if isinstance(wanted, list) else choices.get(qid) == wanted


def _matches(rule: dict, choices: dict) -> bool:
    """A branch rule (``when``) or a verdict rule (``any`` / ``all``) holds for the choices; as jev_judge reads them."""
    if "when" in rule:
        return all(_got(choices, q, c) for q, c in rule["when"].items())
    anyof, allof = rule.get("any") or {}, rule.get("all") or {}
    return (not anyof or any(_got(choices, q, c) for q, c in anyof.items())) and all(_got(choices, q, c) for q, c in allof.items())


def fired_rule(node: dict, per_choices: list) -> tuple:
    """(position, indexes): the 1-based position, in rank order, of the first rule of the wave that matches any request's
    choices, and the indexes (into ``per_choices``) it matched; (None, []) when no rule does. The position is the rule's
    rank only in a JSON tree; ``_rule_rank`` gives a TTL tree's own ``jj:rank``."""
    for position, rule in enumerate(node.get("next") or node.get("verdict_rules") or [], 1):
        hit = [i for i, choices in enumerate(per_choices) if _matches(rule, choices)]
        if hit:
            return position, hit
    return None, []


def _rule_ranks(tree: Any) -> dict:
    """``{wave id: [rank of each rule, in rank order]}`` for a TreeGraph (``jev_graph.rule_ranks``); {} for a JSON dict,
    whose list order is its rank."""
    if isinstance(tree, dict):
        return {}
    import jev_graph
    return jev_graph.rule_ranks(tree)


def run_triples(tree: Any, run: dict, result_lines: list, walk: dict) -> list:
    """The triples of one record's run, as pyoxigraph Quads in the default graph, sorted.

    ``tree``: the quality JSON dict, or a TreeGraph. ``run``: ``name`` (the run name), ``tree_id`` unused beyond the tree,
    ``tree_sha256`` (the JSON digest, used for a dict tree), ``record_id`` and ``repository`` (optional; see the module
    docstring), ``definitions`` (optional: the IRIs of the definitions the run's requests carried, each linked from the run
    by ``jj:carriedDefinition``). ``result_lines``: the batch result lines of the record's waves (``request_id``, ``candidate_id`` = the
    node id, ``answers``). ``walk``: one ``results[]`` entry of the ``quality`` output: ``verdict`` and ``waves``
    (``node``, ``request_id`` or, per statement, ``request_ids`` and ``reason`` = [{statement, ...}]).
    """
    import pyoxigraph as ox
    quality = _quality_json(tree)
    iris = _iris(tree, quality)
    ranks = _rule_ranks(tree)
    by_request = {line["request_id"]: line for line in result_lines}
    run_node, rec = ox.NamedNode(run_iri(run)), ox.NamedNode(record_iri(run))
    quads: set = set()

    def add(s, p, o):
        quads.add(ox.Quad(s, ox.NamedNode(p if p == RDF_TYPE else JJ + p), o, ox.DefaultGraph()))

    def literal(value) -> Any:
        return ox.Literal(repr(float(value)), datatype=ox.NamedNode(XSD_DOUBLE))

    add(run_node, RDF_TYPE, ox.NamedNode(JJ + "Run"))
    add(run_node, "tree", ox.NamedNode(iris.tree()))
    add(run_node, "treeDigest", ox.Literal(_digest(tree, run)))
    add(run_node, "record", rec)
    add(run_node, "verdictValue", ox.Literal(walk["verdict"]))
    for definition in run.get("definitions") or ():  # ADR-JEV-0008: the definitions the run's requests carried
        add(run_node, "carriedDefinition", ox.NamedNode(definition))
    for wave in walk["waves"]:
        node_id = wave["node"]
        node = quality["nodes"][node_id]
        per = wave.get("per") == "statement"
        ids = wave["request_ids"] if per else [wave["request_id"]]
        statements = [r["statement"] for r in wave["reason"]] if per else [None]
        lines = [by_request[i] for i in ids]
        per_choices = [{qid: _choice(a)[0] for qid, a in line["answers"].items()} for line in lines]
        position, hit = fired_rule(node, per_choices)
        rank = None if position is None else (ranks[node_id][position - 1] if node_id in ranks else position)
        for i, (request_id, line, statement) in enumerate(zip(ids, lines, statements)):
            wave_run = ox.NamedNode(f"urn:judge-jev:waverun:{run['name']}:{request_id}")
            add(wave_run, RDF_TYPE, ox.NamedNode(JJ + "WaveRun"))
            add(wave_run, "inRun", run_node)
            add(wave_run, "node", ox.NamedNode(iris.node(node_id)))
            add(wave_run, "requestId", ox.Literal(request_id))
            if statement is not None:
                add(wave_run, "statement", ox.Literal(statement["text"]))
            if position is not None and i in hit:
                rule_node = ox.NamedNode(iris.rule(node_id, position))
                add(wave_run, "fired", rule_node)
                add(rule_node, "rank", ox.Literal(str(rank), datatype=ox.NamedNode(XSD_INTEGER)))  # the tree's own jj:rank of the rule
            for qid, ans in line["answers"].items():
                choice, confidence = _choice(ans)
                answer = ox.NamedNode(f"urn:judge-jev:answer:{run['name']}:{request_id}:{qid}")
                add(answer, RDF_TYPE, ox.NamedNode(JJ + "Answer"))
                add(answer, "ofWaveRun", wave_run)
                add(answer, "toQuestion", ox.NamedNode(iris.question(node_id, qid)))
                add(answer, "chose", ox.NamedNode(iris.option(node_id, qid, choice)))
                add(answer, "confidence", literal(confidence))
                for label, p in sorted((ans.get("probabilities") or {}).items()):
                    prob = ox.NamedNode(f"{answer.value}:{quote(label, safe='')}")
                    add(answer, "probability", prob)
                    add(prob, RDF_TYPE, ox.NamedNode(JJ + "OptionProbability"))
                    add(prob, "option", ox.NamedNode(iris.option(node_id, qid, label)))
                    add(prob, "p", literal(p))
    return sorted(quads, key=str)


def _plain(value):
    """An sc-observability stored value as plain JSON data (its own ``sc_observability.*`` fields dropped)."""
    if hasattr(value, "items"):
        return {k: _plain(v) for k, v in value.items() if not k.startswith("sc_observability.")}
    if isinstance(value, (tuple, list)):
        return [_plain(v) for v in value]
    if getattr(value, "kind", None) == "null":
        return None
    if hasattr(value, "kind") and hasattr(value, "value"):
        return _plain(value.value)
    return value


def read_run_events(log_root, run_name: str) -> Result[dict, Any]:
    """The run's ``result``, ``statement``, ``wave`` and ``verdict`` events, oldest first, through sc-observability's query
    API: {action: [fields]}. One query per action, matched on the run name; a query holds at most LOG_LIMIT events, so a
    run with more of one action is refused rather than silently cut."""
    import sc_observability as sco
    created = sco.create_logger(sco.LoggerConfig(service=LOG_SERVICE, log_root=str(log_root)))
    if isinstance(created, sco.Err):
        return _log_error(f"could not open the log under {log_root}: {created.error}")
    logger, out = created.value, {}
    try:
        for action in ("result", "statement", "wave", "verdict"):
            got = logger.query(sco.LogQuery(service=LOG_SERVICE, action=action, limit=LOG_LIMIT,
                                            field_matches=(sco.FieldMatch(field="run", value=run_name),)))
            if isinstance(got, sco.Err):
                return _log_error(f"log query failed: {got.error}")
            if len(got.value.events) >= LOG_LIMIT:
                return _log_error(f"run {run_name} has {LOG_LIMIT} or more {action} events; one query returns at most {LOG_LIMIT}")
            out[action] = [_plain(e.fields) for e in got.value.events]
    finally:
        logger.shutdown()
    return Ok(out)


def _log_error(message: str) -> Err:
    from jev_judge import JudgeError
    return Err(JudgeError("LOG.READ", message, action="Pass the --run-dir of a finished quality run."))


def rebuild_from_log(log_root, run_name: str, tree: Any, repository: Optional[str] = None) -> Result[list, Any]:
    """The run's triples (every record under the run name) rebuilt from the sc-observability log alone: its ``result``
    events are the result lines, its ``statement`` and ``wave`` events the walk, its ``verdict`` event the verdict.
    ``repository`` is the Raptor repository URN when the log does not carry it (it does not today)."""
    events = read_run_events(log_root, run_name)
    if isinstance(events, Err):
        return events
    records: dict = {}

    def record(fields):
        return records.setdefault(fields.get("record_id"), {"lines": [], "statements": {}, "waves": [], "verdict": None, "run": {}})

    for fields in events.value["result"]:
        rec = record(fields)
        rec["lines"].append(fields)
        rec["run"] = {"name": run_name, "tree_sha256": fields.get("tree_sha256"), "record_id": fields.get("record_id"),
                      "repository": fields.get("repository") or repository,
                      "definitions": sorted(d["iri"] for d in fields.get("definitions") or ())}
    try:
        for fields in events.value["statement"]:
            record(fields)["statements"][fields["request_id"]] = fields["statement"]
        for fields in events.value["wave"]:
            record(fields)["waves"].append(fields)
        for fields in events.value["verdict"]:
            record(fields)["verdict"] = fields["verdict"]
    except (KeyError, TypeError) as exc:
        return _log_error(f"run {run_name}: a log event lacks a field: {type(exc).__name__}: {exc}")
    quads: list = []
    for record_id, rec in records.items():
        if record_id is None or rec["verdict"] is None:
            continue  # a record whose walk did not finish has no verdict to project
        try:
            waves = []
            for w in rec["waves"]:
                if w.get("per") == "statement":
                    waves.append({"node": w["node"], "per": "statement", "request_ids": w["request_ids"],
                                  "reason": [{"statement": rec["statements"][i]} for i in w["request_ids"]]})
                else:
                    waves.append({"node": w["node"], "request_id": w["request_id"]})
            quads += run_triples(tree, rec["run"], rec["lines"], {"verdict": rec["verdict"], "waves": waves})
        except Exception as exc:  # a log line missing a field, or the log and the tree disagree: a typed error, never a crash
            return _log_error(f"run {run_name} record {record_id}: the log does not match the tree or lacks a field: "
                              f"{type(exc).__name__}: {exc}")
    return Ok(sorted(set(quads), key=str))
