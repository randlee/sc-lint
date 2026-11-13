"""The graph store port, tree side (ADR-JEV-0006, ADR-JEV-0007): a quality tree written in Turtle.

A tree is TTL against the judge-jev ontology (``assets/ontology/judge-jev.ttl``, prefix
``jj: <urn:judge-jev:ontology#>``): waves, questions, options and rules are resources linked by edges, and a question
is one resource with an IRI that any wave may ask. This module is imported by ``jev_judge.py``'s directory and has
exactly this API:

  - ``load_tree(path)``: parse the TTL into a ``TreeGraph`` (``INPUT.MALFORMED`` when it does not parse).
  - ``validate_tree(tree)``: SHACL (``assets/shapes/quality-tree.shacl.ttl``); ``QUALITY.TREE`` listing every violation
    with its focus node, path, shape and message.
  - ``tree_to_quality_json(tree)``: the equivalent JSON quality tree (today's format), rules and questions by rank, so
    the existing walker and the pydantic models run on it unchanged. The tree must have passed ``validate_tree``.
  - ``compile_wave(tree, node_id)``: that wave's JSON ``questions`` object, exactly as sent to Jev.
  - ``tree_digest(tree)``: ``Ok(SHA-256 of the tree's RDFC-1.0 canonical N-Quads)``, or the ``QUALITY.TREE`` error of a
    tree that does not validate (the digest never runs over a graph the shapes have not accepted).
  - ``iri_index(tree)``: the tree graph's own IRIs for its tree, nodes, questions, options and rules, which run triples use.
  - ``rule_ranks(tree)``: ``{wave id: [jj:rank of each rule, in rank order]}``. The compiled JSON lists rules in that
    order without their ranks, and the shapes require ranks to be unique, not consecutive, so a rule's position is not
    its rank.
  - ``load_definitions(path)``, ``validate_definitions(defs)``, ``definition_rows(defs)``: a definitions file (ADR-JEV-0008),
    the same graph machinery over ``jj:Definition`` and ``jj:Attachment`` with ``assets/shapes/definitions.shacl.ttl``.
    A definitions file is its own graph: it is never loaded into a tree's store, so it cannot change a tree's digest.

The ontology defines the graph and validates its edges (the shapes are closed: a tree file holds only the shaped ``jj:``
classes, so a blank node can only be a condition); the pydantic models verify the body of each node. A tree file is
limited to ``MAX_TREE_BYTES`` and is checked before it is parsed.

``TreeGraph.store`` is the ``pyoxigraph.Store`` holding the tree, for queries. The SHACL validator (pyshacl) reads
rdflib graphs, not an Oxigraph store: ``validate_tree`` serialises the store to N-Triples and parses that into rdflib,
for validation only. Everything else uses the store.
"""

from __future__ import annotations

import contextlib
import hashlib
import logging
import os
import pathlib
import re
import sys
import warnings
from dataclasses import dataclass, field
from typing import Any, Optional

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import pyoxigraph as ox  # noqa: E402
from result import Err, Ok, Result  # noqa: E402
from jev_errors import JudgeError  # noqa: E402

HERE = pathlib.Path(__file__).resolve().parent
SHAPES_PATH = HERE.parent / "assets" / "shapes" / "quality-tree.shacl.ttl"
JJ = "urn:judge-jev:ontology#"
JJS = "urn:judge-jev:shapes#"
TREE_SCHEMA = "judge-jev/quality-tree/1"
SHAPES_DEFINITIONS_PATH = HERE.parent / "assets" / "shapes" / "definitions.shacl.ttl"
MAX_LISTED_VIOLATIONS = 25
MAX_TREE_BYTES = 128 * 1024  # a tree file over 128 KiB is refused before it is parsed: shipped trees are 4 KiB, and shape checking grows with the square of a wave's questions (a 900 KiB wave took 155 s)
MAX_DEFINITIONS_BYTES = MAX_TREE_BYTES  # a definitions file takes the tree limit and is refused before it is parsed


def _fail(code: str, message: str, action: str = "", data: Optional[dict] = None) -> Err[JudgeError]:
    error = JudgeError(code, message, action=action)
    error.data = data
    return Err(error)


@dataclass(frozen=True)
class TreeGraph:
    """A parsed quality tree: ``store`` is the pyoxigraph store holding its triples (the default graph).

    ``state`` records that ``validate_tree`` accepted the graph (set once; the store is not changed afterwards).
    """

    store: ox.Store
    path: str
    state: dict = field(default_factory=dict, compare=False, repr=False)


class TreeInvalid(Exception):
    """Raised by ``tree_to_quality_json`` and ``compile_wave`` for a graph that does not validate; ``.error`` is the Err's JudgeError."""

    def __init__(self, error: "JudgeError"):
        super().__init__(error.message)
        self.error = error


def _jj(name: str) -> ox.NamedNode:
    return ox.NamedNode(JJ + name)


def _load_ttl(path: Any, what: str, limit: int, size_code: str, action_file: str) -> Result[TreeGraph, JudgeError]:
    """The shared loader of a tree file and a definitions file: size, then UTF-8, then Turtle; every failure typed."""
    if not isinstance(path, (str, os.PathLike)):
        return _fail("INPUT.MISSING", f"{what} path must be a str or a path, got {type(path).__name__}",
                     f"Pass the path of an existing {action_file}.")
    try:
        p = pathlib.Path(path)
        found = bool(str(path).strip()) and p.is_file()
    except (OSError, ValueError):
        found = False
    if not found:
        return _fail("INPUT.MISSING", f"{what} not found: {str(path)!r}", f"Pass the path of an existing {action_file}.")
    try:
        with open(p, "rb") as handle:
            raw = handle.read(limit + 1)
    except OSError as exc:
        return _fail("INPUT.MALFORMED", f"{what} {path} is not readable: {type(exc).__name__}: {exc}", f"Check {path}.")
    if len(raw) > limit:
        return _fail(size_code, f"{what} {path} is larger than the {limit} byte limit",
                     f"Split the file; a {action_file} is at most {limit} bytes.")
    try:
        store = ox.Store()
        store.load(input=raw.decode("utf-8"), format=ox.RdfFormat.TURTLE)
    except (OSError, ValueError, SyntaxError, RecursionError) as exc:  # UnicodeDecodeError is a ValueError
        return _fail("INPUT.MALFORMED", f"{what} {path} is not readable Turtle: {type(exc).__name__}: {exc}",
                     f"Fix the Turtle in {path}.")
    return Ok(TreeGraph(store, str(path)))


def load_tree(path: Any) -> Result[TreeGraph, JudgeError]:
    """Parse the TTL file at ``path`` into a TreeGraph. Nothing is checked beyond the size and the syntax: see ``validate_tree``.

    ``path`` is a str or a path-like; anything else (None, bytes, a number) is ``INPUT.MISSING``. A file over
    ``MAX_TREE_BYTES`` is ``QUALITY.TREE``, refused before it is parsed.
    """
    return _load_ttl(path, "quality tree", MAX_TREE_BYTES, "QUALITY.TREE", "tree file")


def load_definitions(path: Any) -> Result[TreeGraph, JudgeError]:
    """Parse the definitions file at ``path`` into a TreeGraph (its own store, never a tree's). Same rules as ``load_tree``:
    ``INPUT.MISSING`` for a missing path, ``INPUT.MALFORMED`` for non-UTF-8 or unparsable Turtle, and ``DEFINITIONS.INVALID``
    for a file over ``MAX_DEFINITIONS_BYTES``, refused before it is parsed."""
    return _load_ttl(path, "definitions file", MAX_DEFINITIONS_BYTES, "DEFINITIONS.INVALID", "definitions file")


@contextlib.contextmanager
def _rdflib_log():
    """Collect what rdflib and pyshacl log (for example the traceback of an ill-typed literal) instead of letting it reach stderr.

    Yields a list that fills with one ``"<logger>: <message> (<exception type>: <exception>)"`` text per record.
    """
    records: list = []

    class Collect(logging.Handler):
        def emit(self, record):
            text = f"{record.name}: {record.getMessage()}"
            if record.exc_info and record.exc_info[1] is not None:
                text += f" ({type(record.exc_info[1]).__name__}: {record.exc_info[1]})"
            records.append(text)

    handler, saved = Collect(level=logging.DEBUG), []
    for name in ("rdflib", "pyshacl"):
        logger = logging.getLogger(name)
        saved.append((logger, logger.propagate, logger.level))
        logger.addHandler(handler)
        logger.propagate = False
        logger.setLevel(logging.WARNING)
    try:
        with warnings.catch_warnings(record=True) as caught:  # rdflib also warns, e.g. "Parsing weird boolean"
            warnings.simplefilter("always")
            yield records
            records.extend(f"{w.category.__name__}: {w.message}" for w in caught)
    finally:
        for logger, propagate, level in saved:
            logger.removeHandler(handler)
            logger.propagate = propagate
            logger.setLevel(level)


# --------------------------------------------------------------------------- #
# reading the graph
# --------------------------------------------------------------------------- #
def _objects(tree: TreeGraph, subject, name: str) -> list:
    return [q.object for q in tree.store.quads_for_pattern(subject, _jj(name), None, ox.DefaultGraph())]


def _value(term):
    """A literal as a Python str, int or bool; an IRI as its string."""
    if isinstance(term, ox.Literal):
        datatype = term.datatype.value
        if datatype.endswith("#integer"):
            return int(term.value)
        if datatype.endswith("#boolean"):
            return term.value in ("true", "1")
    return term.value


def _one(tree: TreeGraph, subject, name: str, default=None):
    found = _objects(tree, subject, name)
    return _value(found[0]) if found else default


def _by_rank(tree: TreeGraph, terms: list) -> list:
    return sorted(terms, key=lambda t: (_one(tree, t, "rank", 0), str(t)))


def _trees(tree: TreeGraph) -> list:
    rdf_type = ox.NamedNode("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
    return sorted((q.subject for q in tree.store.quads_for_pattern(None, rdf_type, _jj("Tree"), ox.DefaultGraph())), key=str)


def _is_a(tree: TreeGraph, subject, cls: str) -> bool:
    rdf_type = ox.NamedNode("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
    return any(True for _ in tree.store.quads_for_pattern(subject, rdf_type, _jj(cls), ox.DefaultGraph()))


def _question_json(tree: TreeGraph, question) -> dict:
    options = _by_rank(tree, _objects(tree, question, "option"))
    return {"type": "choice", "instructions": _one(tree, question, "instructions"),
            "criteria": {_one(tree, o, "label"): _one(tree, o, "criterion") for o in options}}


def _questions_json(tree: TreeGraph, wave) -> dict:
    return {_one(tree, q, "id"): _question_json(tree, q) for q in _by_rank(tree, _objects(tree, wave, "asks"))}


def _conditions_json(tree: TreeGraph, rule, prop: str, intersect: bool) -> dict:
    """``{question id: choice or [choices]}``, questions by rank: one choice as a string, several as a list in option
    rank order. Two conditions on one question are merged (union for whenAny, intersection for whenAll)."""
    merged: dict = {}
    for condition in _objects(tree, rule, prop):
        question = _objects(tree, condition, "question")[0]
        choices = set(_objects(tree, condition, "choice"))
        merged[question] = merged[question] & choices if intersect and question in merged else merged.get(question, set()) | choices
    out = {}
    for question in _by_rank(tree, list(merged)):
        labels = [_one(tree, o, "label") for o in _by_rank(tree, list(merged[question]))]
        out[_one(tree, question, "id")] = labels[0] if len(labels) == 1 else labels
    return out


def _rules_json(tree: TreeGraph, wave) -> tuple:
    rules = _by_rank(tree, _objects(tree, wave, "rule"))
    if rules and _is_a(tree, rules[0], "BranchRule"):
        out = []
        for rule in rules:
            item: dict = {}
            when = _conditions_json(tree, rule, "whenAll", True)
            if when:
                item["when"] = when
            item["goto"] = _one(tree, _objects(tree, rule, "goto")[0], "id")
            out.append(item)
        return "next", out
    out = []
    for rule in rules:
        item = {}
        for key, prop, intersect in (("any", "whenAny", False), ("all", "whenAll", True)):
            conditions = _conditions_json(tree, rule, prop, intersect)
            if conditions:
                item[key] = conditions
        item["verdict"] = _one(tree, rule, "verdict")
        out.append(item)
    return "verdict_rules", out


def _node_json(tree: TreeGraph, node) -> dict:
    if _is_a(tree, node, "Leaf"):
        return {"verdict": _one(tree, node, "verdict")}
    out: dict = {}
    if _one(tree, node, "per") is not None:
        out["per"] = _one(tree, node, "per")
        out["statements"] = _one(tree, node, "statements")
    modals = sorted(_value(m) for m in _objects(tree, node, "modal"))
    if modals:
        out["modals"] = modals
    out["questions"] = _questions_json(tree, node)
    key, rules = _rules_json(tree, node)
    out[key] = rules
    return out


def _ensure_valid(tree: TreeGraph) -> None:
    """Validate once (cached on the tree); raise ``TreeInvalid`` when the graph does not conform."""
    if tree.state.get("valid"):
        return
    result = validate_tree(tree)
    if isinstance(result, Err):
        raise TreeInvalid(result.error)


def tree_to_quality_json(tree: TreeGraph) -> dict:
    """The JSON quality tree equivalent to the graph, in the format of ``assets/questions/quality-*.json``.

    The graph is validated first (once); raises ``TreeInvalid`` (``.error`` is the ``QUALITY.TREE`` JudgeError) when it does not conform.
    """
    _ensure_valid(tree)
    root = _trees(tree)[0]
    out: dict = {"schema": TREE_SCHEMA, "id": _one(tree, root, "id")}
    for key in ("kind", "version", "approved", "note"):
        found = _one(tree, root, key)
        if found is not None:
            out[key] = found
    out["start"] = _one(tree, _objects(tree, root, "start")[0], "id")
    nodes = _objects(tree, root, "node")
    out["nodes"] = {_one(tree, n, "id"): _node_json(tree, n) for n in sorted(nodes, key=lambda n: _one(tree, n, "id"))}
    return out


def compile_wave(tree: TreeGraph, node_id: str) -> dict:
    """The ``questions`` object of wave ``node_id``, questions in rank order, exactly as sent to Jev.

    Raises ``KeyError`` when the tree has no wave with that id (a bug in the caller: the walker takes ids from the tree)
    and ``TreeInvalid`` when the graph does not validate. ``OxigraphGraphStore.compile_wave`` turns both into an ``Err``.
    """
    _ensure_valid(tree)
    for node in _objects(tree, _trees(tree)[0], "node"):
        if _one(tree, node, "id") == node_id and _is_a(tree, node, "Wave"):
            return _questions_json(tree, node)
    raise KeyError(f"no wave {node_id!r} in the tree")


class GraphIris:
    """The tree graph's own IRIs, by the ids the walker and the log use.

    ``tree()``; ``node(node_id)``; ``question(node_id, question_id)``; ``option(node_id, question_id, label)``;
    ``rule(node_id, position)`` with ``position`` the rule's 1-based position in rank order (as ``jev_runs.fired_rule``
    counts; ``rule_ranks`` gives its ``jj:rank``).
    A name the graph does not hold is a ``KeyError``. No naming scheme is assumed: the IRIs are whatever the tree file says.
    """

    def __init__(self, tree: TreeGraph):
        root = _trees(tree)[0]
        self._tree = root.value
        self._nodes, self._questions, self._options, self._rules = {}, {}, {}, {}
        for node in _objects(tree, root, "node"):
            node_id = _one(tree, node, "id")
            self._nodes[node_id] = node.value
            for position, rule in enumerate(_by_rank(tree, _objects(tree, node, "rule")), 1):
                self._rules[(node_id, position)] = rule.value
            for question in _objects(tree, node, "asks"):
                qid = _one(tree, question, "id")
                self._questions[(node_id, qid)] = question.value
                for option in _objects(tree, question, "option"):
                    self._options[(node_id, qid, _one(tree, option, "label"))] = option.value

    def tree(self) -> str:
        return self._tree

    def node(self, node_id: str) -> str:
        return self._nodes[node_id]

    def question(self, node_id: str, question_id: str) -> str:
        return self._questions[(node_id, question_id)]

    def option(self, node_id: str, question_id: str, label: str) -> str:
        return self._options[(node_id, question_id, label)]

    def rule(self, node_id: str, position: int) -> str:
        return self._rules[(node_id, position)]


def rule_ranks(tree: TreeGraph) -> dict:
    """``{wave id: [jj:rank of each rule, in rank order]}`` of a valid tree (``TreeInvalid`` when it does not validate)."""
    _ensure_valid(tree)
    out = {}
    for node in _objects(tree, _trees(tree)[0], "node"):
        rules = _by_rank(tree, _objects(tree, node, "rule"))
        if rules:
            out[_one(tree, node, "id")] = [_one(tree, rule, "rank") for rule in rules]
    return out


def iri_index(tree: TreeGraph) -> GraphIris:
    """The ``GraphIris`` of a valid tree (``TreeInvalid`` when the graph does not validate)."""
    _ensure_valid(tree)
    return GraphIris(tree)


def tree_digest(tree: TreeGraph) -> Result[str, JudgeError]:
    """``Ok`` of the SHA-256 (lower-case hex) of the tree's RDFC-1.0 canonical N-Quads, one quad per line in sorted order.

    The tree is validated first (once): the shapes are closed, every subject is a shaped class, every ``rdf:type`` object
    is one of those classes and only conditions may be blank nodes, so canonicalization cannot meet a deep blank-node
    chain. A tree that does not validate is its ``QUALITY.TREE`` ``Err``.
    Blank nodes are relabelled by pyoxigraph's RDFC-1.0 (``CanonicalizationAlgorithm.RDFC_1_0``), so triple order,
    prefixes, whitespace and blank node names do not change the digest; any change to a triple does.
    """
    checked = validate_tree(tree) if not tree.state.get("valid") else Ok(None)
    if isinstance(checked, Err):
        return checked
    dataset = ox.Dataset(tree.store.quads_for_pattern(None, None, None, ox.DefaultGraph()))  # the tree's own graph: a named graph (definitions) never changes the digest
    dataset.canonicalize(ox.CanonicalizationAlgorithm.RDFC_1_0)
    lines = sorted(line for line in ox.serialize(dataset, format=ox.RdfFormat.N_QUADS).decode("utf-8").splitlines() if line)
    return Ok(hashlib.sha256(("\n".join(lines) + "\n").encode("utf-8")).hexdigest())


# --------------------------------------------------------------------------- #
# SHACL
# --------------------------------------------------------------------------- #
BNODE_TEXT = re.compile(r"_:[A-Za-z0-9]+")


def _short(term: Any) -> str:
    text = str(term)
    return text.replace(JJ, "jj:").replace(JJS, "jjs:")


def _describe(graph, term, depth: int = 0) -> str:
    """A run-to-run stable text for a term: an IRI or literal as is; a blank node as ``blank node <predicate> of <parent>``,
    the parent chosen as the smallest in text (never the blank node's label, which differs between parses)."""
    from rdflib import BNode

    if term is None:
        return ""
    if not isinstance(term, BNode):
        return _short(term)
    if depth > 3:
        return "blank node"
    parents = sorted(f"{_short(p)} of {_describe(graph, s, depth + 1)}" for s, p, _o in graph.triples((None, None, term)))
    return f"blank node ({parents[0]})" if parents else "blank node"


def _path_text(graph, path) -> str:
    from rdflib import BNode
    from rdflib.namespace import SH

    if isinstance(path, BNode):
        inverse = graph.value(path, SH.inversePath)
        if inverse is not None:
            return "^" + _short(inverse)
    return _short(path) if path is not None else ""


def _violations(report, data, with_severity: bool = False) -> list:
    """The violations, sorted by their text. No rdflib blank node label is ever printed: a blank focus node or value is
    described by where it hangs in the graph, a property shape (a blank node) by its path and constraint component."""
    from rdflib import BNode
    from rdflib.namespace import RDF, SH

    out = []
    for result in report.subjects(RDF.type, SH.ValidationResult):
        message = report.value(result, SH.resultMessage)
        focus_term, value_term = report.value(result, SH.focusNode), report.value(result, SH.value)
        shape_term, constraint_term = report.value(result, SH.sourceShape), report.value(result, SH.sourceConstraint)
        text = str(message) if message is not None else ""
        for term in (focus_term, value_term):
            if isinstance(term, BNode):
                text = text.replace(str(term), _describe(data, term))
        severity = report.value(result, SH.resultSeverity)
        out.append({
            **({"severity": _short(severity).rsplit("#", 1)[-1].lower()} if with_severity else {}),
            "focus": _describe(data, focus_term),
            "path": _path_text(report, report.value(result, SH.resultPath)),
            "shape": "" if shape_term is None or isinstance(shape_term, BNode) else _short(shape_term),
            "constraint": "" if constraint_term is None or isinstance(constraint_term, BNode) else _short(constraint_term),
            "component": _short(report.value(result, SH.sourceConstraintComponent)).rsplit("#", 1)[-1],
            "value": _describe(data, value_term),
            "message": BNODE_TEXT.sub("blank node", text),
        })
    return sorted(out, key=lambda v: (v["focus"], v["path"], v["component"], v["constraint"], v["shape"], v["value"], v["message"],
                                      v.get("severity", "")))


def _violation_text(v: dict) -> str:
    source = v["constraint"] or f"{v['shape']} {v['component']}".strip()
    where = f" path {v['path']}" if v["path"] else ""
    return f"{v['focus']}{where} [{source}]: {v['message']}".strip()


SHAPED_CLASSES = ("Tree", "Wave", "Leaf", "Question", "Option", "BranchRule", "VerdictRule", "Condition")
DEFINITION_CLASSES = ("Definition", "Attachment")


def _unshaped_query(classes: tuple) -> str:
    return ("PREFIX jj: <%s> SELECT DISTINCT ?s WHERE { ?s ?p ?o . FILTER NOT EXISTS { ?s a ?c . FILTER (?c IN (%s)) } }"
            % (JJ, ", ".join("jj:" + c for c in classes)))


def _stray_type_query(classes: tuple) -> str:
    return "PREFIX jj: <%s> SELECT DISTINCT ?c WHERE { ?s a ?c . FILTER (?c NOT IN (%s)) }" % (JJ, ", ".join("jj:" + c for c in classes))


UNSHAPED_QUERY = _unshaped_query(SHAPED_CLASSES)
STRAY_TYPE_QUERY = _stray_type_query(SHAPED_CLASSES)


def _describe_ox(store: ox.Store, term, depth: int = 0) -> str:
    """``_describe`` over the pyoxigraph store: a blank node by where it hangs, never by its label."""
    if not isinstance(term, ox.BlankNode):
        return _short(term.value)
    if depth > 3:
        return "blank node"
    parents = sorted(f"{_short(q.predicate.value)} of {_describe_ox(store, q.subject, depth + 1)}"
                     for q in store.quads_for_pattern(None, None, term, ox.DefaultGraph()))
    return f"blank node ({parents[0]})" if parents else "blank node"


def _unshaped_subjects(tree: TreeGraph, classes: tuple = SHAPED_CLASSES) -> list:
    """Descriptions (sorted) of the subjects that are none of the shaped ``jj:`` classes: the ``jjs:AllSubjectsShaped`` rule.

    Run on the Oxigraph store before the SHACL pass: the shapes state the same rule, but pyshacl over a graph of
    tens of thousands of stray subjects (a 20000-deep blank-node chain took 16 s, and the time grows with the square of
    the size) is not worth waiting for when the answer is already known.
    """
    query = UNSHAPED_QUERY if classes == SHAPED_CLASSES else _unshaped_query(classes)
    return sorted(_describe_ox(tree.store, row["s"]) for row in tree.store.query(query))


def _stray_types(tree: TreeGraph, classes: tuple = SHAPED_CLASSES) -> list:
    """Descriptions (sorted) of the ``rdf:type`` objects that are none of the shaped ``jj:`` classes: the
    ``jjs:TypesAreShapedClasses`` rule. Closed shapes ignore ``rdf:type``, so without it a chain of valid conditions linked
    through their type triples (``_:i a jj:Condition, _:i+1``) would pass and make RDFC-1.0 canonicalization superlinear."""
    query = STRAY_TYPE_QUERY if classes == SHAPED_CLASSES else _stray_type_query(classes)
    return sorted(_describe_ox(tree.store, row["c"]) for row in tree.store.query(query))


def validate_tree(tree: TreeGraph) -> Result[None, JudgeError]:
    """Ok(None) when the graph conforms to the shapes; else ``QUALITY.TREE`` with each violation (``data["violations"]``).

    What rdflib logs while reading or checking the graph (an ill-typed literal, say) is not printed: it is appended to the
    error text and kept in ``data["log"]``. A conforming graph is remembered as valid on the ``TreeGraph``.
    """
    action = "Fix the tree's Turtle; see assets/questions/quality-adr.ttl for the shape and assets/shapes/quality-tree.shacl.ttl for the rules."
    trees = _trees(tree)
    if len(trees) != 1:
        return _fail("QUALITY.TREE", f"quality tree {tree.path}: the file must define exactly one jj:Tree, it defines {len(trees)}", action)
    subject_message = ("is a subject of the graph but not a jj:Tree, Wave, Leaf, Question, Option, BranchRule, VerdictRule or "
                       "Condition; a tree file holds nothing else")
    type_message = ("is an rdf:type object but not jj:Tree, Wave, Leaf, Question, Option, BranchRule, VerdictRule or "
                    "Condition; a tree file types its nodes with those classes only")
    violations = [{"focus": name, "path": "", "shape": "jjs:TreeShape", "constraint": constraint,
                   "component": "SPARQLConstraintComponent", "value": name, "message": f"{name} {message}"}
                  for constraint, message, names in (("jjs:AllSubjectsShaped", subject_message, _unshaped_subjects(tree)),
                                                     ("jjs:TypesAreShapedClasses", type_message, _stray_types(tree)))
                  for name in names]
    if violations:
        listed = "; ".join(f"{v['focus']} [{v['constraint']}]: {v['message']}" for v in violations[:MAX_LISTED_VIOLATIONS])
        more = f"; and {len(violations) - MAX_LISTED_VIOLATIONS} more" if len(violations) > MAX_LISTED_VIOLATIONS else ""
        return _fail("QUALITY.TREE", f"quality tree {tree.path}: {len(violations)} SHACL violation(s): {listed}{more}", action,
                     {"violations": violations, "log": []})
    try:
        import pyshacl
        from rdflib import Graph
    except ImportError as exc:
        return _fail("DEPENDENCY.MISSING", f"validating a tree needs pyshacl: {exc}", "Install pyshacl (pip install 'pyshacl>=0.30,<1').")
    with _rdflib_log() as logged:
        data = Graph()
        data.parse(data=tree.store.dump(format=ox.RdfFormat.N_TRIPLES, from_graph=ox.DefaultGraph()), format="nt")
        shapes = Graph()
        shapes.parse(str(SHAPES_PATH), format="turtle")
        conforms, report, _text = pyshacl.validate(data, shacl_graph=shapes, inference="none", abort_on_first=False,
                                                   allow_infos=False, allow_warnings=False, meta_shacl=False)
        if not conforms and not hasattr(report, "subjects"):  # pyshacl returns (not raises) a ValidationFailure for a shapes problem
            return _fail("SCRIPT.INTERNAL", f"the SHACL shapes could not be applied: {getattr(report, 'message', report)}",
                         "Report this: assets/shapes/quality-tree.shacl.ttl is broken.")
        violations = [] if conforms else _violations(report, data)
    if conforms:
        tree.state["valid"] = True
        return Ok(None)
    listed = "; ".join(_violation_text(v) for v in violations[:MAX_LISTED_VIOLATIONS])
    more = f"; and {len(violations) - MAX_LISTED_VIOLATIONS} more" if len(violations) > MAX_LISTED_VIOLATIONS else ""
    log = sorted(set(logged))
    noted = f" (rdflib reported: {'; '.join(log)})" if log else ""
    return _fail("QUALITY.TREE", f"quality tree {tree.path}: {len(violations)} SHACL violation(s): {listed}{more}{noted}", action,
                 {"violations": violations, "log": log})


# --------------------------------------------------------------------------- #
# definitions (ADR-JEV-0008)
# --------------------------------------------------------------------------- #
def validate_definitions(defs: TreeGraph) -> Result[list, JudgeError]:
    """``Ok`` of the warnings (a list of violation dicts with ``severity`` "warning", sorted) when the definitions graph has
    no violation; else ``DEFINITIONS.INVALID`` with every violation in ``data["violations"]``.

    Two rules run on the Oxigraph store before SHACL, as for trees: every subject is a ``jj:Definition`` or ``jj:Attachment``
    and every ``rdf:type`` object is one of those two. Then ``assets/shapes/definitions.shacl.ttl``: closed shapes, a
    definition is an IRI with one term, text and source, an attachment has exactly one IRI record and at most one location.
    A definition with no attachment is ``sh:Warning``: the file stays valid and the warning is returned.
    """
    action = "Fix the definitions file; see assets/shapes/definitions.shacl.ttl for the rules."
    subject_message = "is a subject of the graph but not a jj:Definition or jj:Attachment; a definitions file holds nothing else"
    type_message = "is an rdf:type object but not jj:Definition or jj:Attachment; a definitions file types its nodes with those classes only"
    violations = [{"severity": "violation", "focus": name, "path": "", "shape": "jjs:DefinitionShape", "constraint": constraint,
                   "component": "SPARQLConstraintComponent", "value": name, "message": f"{name} {message}"}
                  for constraint, message, names in (("jjs:AllSubjectsShaped", subject_message, _unshaped_subjects(defs, DEFINITION_CLASSES)),
                                                     ("jjs:TypesAreShapedClasses", type_message, _stray_types(defs, DEFINITION_CLASSES)))
                  for name in names]
    if violations:
        return _definitions_error(defs, violations, action, [])
    try:
        import pyshacl
        from rdflib import Graph
    except ImportError as exc:
        return _fail("DEPENDENCY.MISSING", f"validating definitions needs pyshacl: {exc}", "Install pyshacl (pip install 'pyshacl>=0.30,<1').")
    with _rdflib_log() as logged:
        data = Graph()
        data.parse(data=defs.store.dump(format=ox.RdfFormat.N_TRIPLES, from_graph=ox.DefaultGraph()), format="nt")
        shapes = Graph()
        shapes.parse(str(SHAPES_DEFINITIONS_PATH), format="turtle")
        conforms, report, _text = pyshacl.validate(data, shacl_graph=shapes, inference="none", abort_on_first=False,
                                                   allow_infos=True, allow_warnings=True, meta_shacl=False)
        if not conforms and not hasattr(report, "subjects"):
            return _fail("SCRIPT.INTERNAL", f"the SHACL shapes could not be applied: {getattr(report, 'message', report)}",
                         "Report this: assets/shapes/definitions.shacl.ttl is broken.")
        found = _violations(report, data, with_severity=True)
    errors = [v for v in found if v["severity"] != "warning"]
    if errors:
        return _definitions_error(defs, errors, action, sorted(set(logged)))
    defs.state["valid"] = True
    return Ok([v for v in found if v["severity"] == "warning"])


def _definitions_error(defs: TreeGraph, violations: list, action: str, log: list) -> Err[JudgeError]:
    listed = "; ".join(_violation_text(v) for v in violations[:MAX_LISTED_VIOLATIONS])
    more = f"; and {len(violations) - MAX_LISTED_VIOLATIONS} more" if len(violations) > MAX_LISTED_VIOLATIONS else ""
    noted = f" (rdflib reported: {'; '.join(log)})" if log else ""
    return _fail("DEFINITIONS.INVALID", f"definitions file {defs.path}: {len(violations)} SHACL violation(s): {listed}{more}{noted}", action,
                 {"violations": violations, "log": log})


def definition_rows(defs: TreeGraph) -> list:
    """The graph of a definitions file that ``validate_definitions`` accepted, as plain rows sorted by IRI:
    ``{"iri", "term", "text", "source", "attachments": [{"record", "location"}]}`` (``location`` None for a record-only
    attachment; attachments sorted by record then location). Bodies are as the file says: the pydantic model checks them."""
    rdf_type = ox.NamedNode("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
    graph = ox.DefaultGraph()

    def one(subject, name):
        found = [q.object for q in defs.store.quads_for_pattern(subject, _jj(name), None, graph)]
        return found[0].value if found else None

    rows = []
    for quad in sorted(defs.store.quads_for_pattern(None, rdf_type, _jj("Definition"), graph), key=lambda q: str(q.subject)):
        node = quad.subject
        attachments = sorted(({"record": one(a.object, "record"), "location": one(a.object, "location")}
                              for a in defs.store.quads_for_pattern(node, _jj("attachedTo"), None, graph)),
                             key=lambda a: (a["record"] or "", a["location"] or ""))
        rows.append({"iri": node.value, "term": one(node, "term"), "text": one(node, "text"), "source": one(node, "source"),
                     "attachments": attachments})
    return rows
