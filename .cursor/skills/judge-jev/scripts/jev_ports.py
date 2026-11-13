"""jev_ports.py: the two ports of ADR-JEV-0007 and their adapters (Python 3.11 floor).

The seam, in three rules:

* Only JSON crosses ``Transport``. Its contract is the batch request and result
  schemas (``assets/schemas/jev-batch-request.schema.json`` and
  ``jev-batch-result.schema.json``), unchanged. The Python core implements it
  now (``PythonCoreTransport``); sc-agent's Jev tool may implement it later.
* The sc-observability run log is the record of a run. A run's triples are a
  projection of its result lines and the tree (``jev_runs.rebuild_from_log``),
  so they can be rebuilt at any time and nothing is lost when the graph store
  moves to the other side of the seam.
* ``GraphStore`` is everything graph: load a tree, validate it, compile a wave
  to its JSON questions, record a run's triples, answer a SPARQL query.
  ``OxigraphGraphStore`` implements it with ``pyoxigraph`` and ``jev_graph``.

This module imports neither ``jev_judge`` nor ``pyoxigraph`` at load: the
Python core is injected as a callable and the graph modules are imported where
they are used, so there is no import cycle and a Transport-only caller needs
no graph dependency.
"""

from __future__ import annotations

import pathlib
import sys
from typing import Any, Callable, Iterable, Optional, Protocol, runtime_checkable

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from result import Err, Ok, Result  # noqa: E402


@runtime_checkable
class Transport(Protocol):
    """Sends the request lines of one wave; returns their result lines, in request order.

    A line in and a line out are the batch request and result JSON objects, unchanged. An ``Err`` carries the
    ``JudgeError`` of a failure that stopped the whole send; a line that got no answer is a result line with
    ``ok`` false and its own ``error``.
    """

    def send(self, lines: list[dict]) -> Result[list[dict], Any]: ...


@runtime_checkable
class GraphStore(Protocol):
    """Loads a quality tree, validates it, compiles a wave, records a run's triples and answers SPARQL."""

    def load_tree(self, path: str) -> Result[Any, Any]: ...

    def validate_tree(self, tree: Any) -> Result[Any, Any]: ...

    def compile_wave(self, tree: Any, node_id: str) -> Result[dict, Any]: ...

    def record_run(self, triples: Iterable[Any]) -> Result[int, Any]: ...

    def query(self, sparql: str) -> Result[list[dict], Any]:
        """Answer a SPARQL SELECT as one dict per solution. Only SELECT is supported: ASK, CONSTRUCT and DESCRIBE are
        refused with a typed ``Err`` (``VALIDATION.INPUT``), as is any query the store cannot parse or evaluate."""
        ...


class PythonCoreTransport:
    """``Transport`` over the Python core: ``pump`` is ``jev_judge.pump_batch``, injected so this module never imports it.

    ``send`` runs ``pump(lines, settings, api_key, fixture, log, echo, fields)``; the settings, key, fixture (an answers
    fixture, or None for a live call), run log and log fields are the caller's, fixed at construction.
    """

    def __init__(self, pump: Callable[..., Result], settings: Any, api_key: str, fixture: Any, log: Any,
                 echo: bool = False, fields: Optional[dict] = None):
        self._pump, self._settings, self._api_key = pump, settings, api_key
        self._fixture, self._log, self._echo, self._fields = fixture, log, echo, fields

    def send(self, lines: list[dict]) -> Result[list[dict], Any]:
        return self._pump(lines, self._settings, self._api_key, self._fixture, self._log, self._echo, self._fields)


class FixtureTransport:
    """``Transport`` for tests: result lines come from ``responses`` and no Jev is called.

    ``responses`` is a callable ``lines -> list of result lines`` or a dict ``request_id -> result line``; the latter
    is keyed by the request id the Python core would give each line (``<prefix>/<item id>/<candidate id>/r<repeat>``),
    so the caller passes the ids it expects. ``calls`` records every ``send``. A line with no fixture is an
    ``INPUT.MISSING`` ``Err``.
    """

    def __init__(self, responses: Callable[[list[dict]], list[dict]] | dict):
        self._responses = responses
        self.calls: list[list[dict]] = []

    def send(self, lines: list[dict]) -> Result[list[dict], Any]:
        self.calls.append(lines)
        if callable(self._responses):
            return Ok(list(self._responses(lines)))
        out = []
        for line in lines:
            key = line.get("request_id") or _request_key(line)
            if key not in self._responses:
                from jev_judge import JudgeError  # lazy: the error type lives with the Python core
                return Err(JudgeError("INPUT.MISSING", f"FixtureTransport has no result line for {key!r}",
                                      action="Add the request id to the fixture's responses."))
            out.append(self._responses[key])
        return Ok(out)


def _request_key(line: dict) -> str:
    item = line.get("item", {})
    candidates = line.get("questions") or [{}]
    return f"{item.get('id')}/{candidates[0].get('candidate_id')}"


class OxigraphGraphStore:
    """``GraphStore`` over ``pyoxigraph``: tree operations delegate to ``jev_graph``; runs and queries use one Store.

    ``path`` None keeps the store in memory; a path persists it there. ``record_run`` adds a run's quads (``jev_runs``
    makes them) and returns how many were new; ``query`` runs a SPARQL SELECT and returns one dict per solution mapping
    each variable to its term's plain value (an IRI, a literal's lexical form); an unbound variable is omitted.
    """

    def __init__(self, path: Optional[str] = None):
        import pyoxigraph
        self.store = pyoxigraph.Store(path) if path else pyoxigraph.Store()

    def load_tree(self, path: str) -> Result[Any, Any]:
        import jev_graph
        return jev_graph.load_tree(path)

    def validate_tree(self, tree: Any) -> Result[Any, Any]:
        import jev_graph
        return jev_graph.validate_tree(tree)

    def compile_wave(self, tree: Any, node_id: str) -> Result[dict, Any]:
        """The wave's JSON ``questions``; an unknown wave or a tree that does not validate is a typed ``Err``."""
        import jev_graph
        try:
            return Ok(jev_graph.compile_wave(tree, node_id))
        except jev_graph.TreeInvalid as exc:
            return Err(exc.error)
        except KeyError:
            from jev_judge import JudgeError
            return Err(JudgeError("VALIDATION.INPUT", f"the tree has no wave {node_id!r}",
                                  action="Pass the id of a wave of the tree."))

    def load_definitions(self, paths: Iterable[str]) -> Result[int, Any]:
        """Load definitions files into this store, each as its own named graph ``urn:judge-jev:definitions:<file name>``
        (never the default graph, so a tree's digest does not change). Every file is validated first
        (``jev_graph.load_definitions`` and ``validate_definitions``); the first bad one is the ``Err``. Returns the number
        of quads added."""
        import jev_graph
        import pyoxigraph
        before = len(self.store)
        for path in paths:
            loaded = jev_graph.load_definitions(path)
            if isinstance(loaded, Err):
                return loaded
            checked = jev_graph.validate_definitions(loaded.value)
            if isinstance(checked, Err):
                return checked
            graph = pyoxigraph.NamedNode("urn:judge-jev:definitions:" + pathlib.Path(path).name)
            self.store.extend(pyoxigraph.Quad(q.subject, q.predicate, q.object, graph)
                              for q in loaded.value.store.quads_for_pattern(None, None, None, pyoxigraph.DefaultGraph()))
        return Ok(len(self.store) - before)

    def record_run(self, triples: Iterable[Any]) -> Result[int, Any]:
        try:
            before = len(self.store)
            self.store.extend(list(triples))
            return Ok(len(self.store) - before)
        except (TypeError, ValueError, OSError, RuntimeError) as exc:
            from jev_judge import JudgeError
            return Err(JudgeError("VALIDATION.INPUT", f"record_run needs pyoxigraph quads: {type(exc).__name__}: {exc}",
                                  action="Pass the quads jev_runs makes."))

    def query(self, sparql: str) -> Result[list[dict], Any]:
        import pyoxigraph
        from jev_judge import JudgeError
        try:
            solutions = self.store.query(sparql)
            if not isinstance(solutions, pyoxigraph.QuerySolutions):
                return Err(JudgeError("VALIDATION.INPUT", "only a SPARQL SELECT query is supported here, not ASK, CONSTRUCT or DESCRIBE",
                                      action="Rewrite the query as a SELECT."))
            variables = [v.value for v in solutions.variables]
            rows = [{name: solution[name].value for name in variables if solution[name] is not None} for solution in solutions]
        except Exception as exc:  # pyoxigraph raises SyntaxError, ValueError, OSError, RuntimeError, TypeError (a non-str query)
            return Err(JudgeError("VALIDATION.INPUT", f"SPARQL query failed: {type(exc).__name__}: {exc}", action="Fix the query."))
        return Ok(rows)
