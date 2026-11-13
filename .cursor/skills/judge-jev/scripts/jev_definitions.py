"""jev_definitions.py: definitions as data that a request carries (ADR-JEV-0008, REQ-JEV-0212), standard library only.

A definition (``jj:Definition``: ``jj:term``, ``jj:text``, ``jj:source``) is attached to a record alone, or to a record and
one location (``jj:attachedTo`` a ``jj:Attachment`` with ``jj:record`` and optionally ``jj:location``). This module holds the
parts that need neither the graph libraries nor pydantic; ``jev_judge.py`` loads and validates the files (``jev_graph``
for the graph, a pydantic model for each body) and hands the validated rows here.

  * ``Definition``: one definition with its attachments and ``digest``, the SHA-256 of the canonical
    ``{"source", "term", "text"}`` JSON (sorted keys, compact separators, UTF-8, no ASCII escaping).
  * ``DefinitionSet.for_request(record_iri, location)``: the definitions a request about that record (at that location)
    carries: the record-only ones, plus the ones attached to that record and exactly that location; sorted by term (then
    IRI), each once.
  * ``state_entries(definitions)``: the ``[{"term", "text"}]`` list that goes into the request's state.
  * ``log_entries(definitions)``: the ``[{"iri", "term", "digest"}]`` list that goes into every log event of the request.
  * ``discover(repository_root)`` and ``scan(directory)``: where definitions files live.

File names: a definitions file is ``definitions.ttl`` or ``<name>.definitions.ttl`` in ``<repository root>/.sc/judge-jev/``.
Every other ``.ttl`` there is a quality tree.
"""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
from dataclasses import dataclass, field
from typing import Optional

DEFINITIONS_DIR = ".sc/judge-jev"
DEFINITIONS_FILE = "definitions.ttl"
DEFINITIONS_SUFFIX = ".definitions.ttl"
SKIPPED_DIRS = ("logs", "anchor-runs", "node_modules", ".git")  # never scanned for trees or definitions


def is_definitions_file(name: str) -> bool:
    return name == DEFINITIONS_FILE or name.endswith(DEFINITIONS_SUFFIX)


def digest(term: str, text: str, source: str) -> str:
    """SHA-256 (lower-case hex) of the canonical JSON of ``{term, text, source}``."""
    canonical = json.dumps({"term": term, "text": text, "source": source}, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


@dataclass(frozen=True)
class Definition:
    iri: str
    term: str
    text: str
    source: str
    attachments: tuple = ()  # ((record IRI, location or None), ...)
    file: str = field(default="", compare=False)

    @property
    def digest(self) -> str:
        return digest(self.term, self.text, self.source)

    def entry(self) -> dict:
        return {"term": self.term, "text": self.text}

    def reference(self) -> dict:
        return {"iri": self.iri, "term": self.term, "digest": self.digest}

    def reaches(self, record_iri: str, location: Optional[str]) -> bool:
        """Record-only attachment: every request about the record. Record and location: only a request at that location."""
        return any(record == record_iri and (at is None or at == location) for record, at in self.attachments)


@dataclass(frozen=True)
class DefinitionSet:
    definitions: tuple = ()
    files: tuple = ()

    def for_request(self, record_iri: str, location: Optional[str] = None) -> list:
        return sorted((d for d in self.definitions if d.reaches(record_iri, location)), key=lambda d: (d.term, d.iri))

    def by_iri(self, iri: str) -> Optional[Definition]:
        return next((d for d in self.definitions if d.iri == iri), None)


def build_set(rows_by_file: list) -> tuple:
    """``(DefinitionSet, None)`` from ``[(file, [row])]`` (rows as ``jev_graph.definition_rows`` makes them, bodies already
    validated), or ``(None, message)`` when two files define the same IRI: one IRI is one definition."""
    seen: dict = {}
    out = []
    for file, rows in rows_by_file:
        for row in rows:
            if row["iri"] in seen:
                return None, f"definition {row['iri']} is defined in both {seen[row['iri']]} and {file}; an IRI names one definition"
            seen[row["iri"]] = file
            attachments = tuple(sorted(((a["record"], a["location"]) for a in row["attachments"]), key=lambda a: (a[0], a[1] or "")))
            out.append(Definition(row["iri"], row["term"], row["text"], row["source"], attachments, file))
    return DefinitionSet(tuple(sorted(out, key=lambda d: d.iri)), tuple(f for f, _rows in rows_by_file)), None


def state_entries(definitions: list) -> list:
    return [d.entry() for d in definitions]


def log_entries(definitions: list) -> list:
    return [d.reference() for d in definitions]


def discover(repository_root) -> list:
    """The definitions files directly in ``<repository root>/.sc/judge-jev/``, sorted; none when the folder is absent."""
    folder = pathlib.Path(repository_root or ".") / DEFINITIONS_DIR
    try:
        return sorted(str(p) for p in folder.iterdir() if p.is_file() and is_definitions_file(p.name))
    except OSError:
        return []


def scan(directory, recursive: bool = True) -> tuple:
    """``(definitions files, tree files)`` under ``directory``, recursively (or, with ``recursive=False``, directly in it) and sorted, skipping ``SKIPPED_DIRS``: a
    ``definitions.ttl`` or ``*.definitions.ttl`` is a definitions file, every other ``.ttl`` a tree. Nothing found, or an
    unreadable directory, is two empty lists."""
    definitions, trees = [], []
    try:
        for current, dirs, files in os.walk(directory):
            dirs[:] = sorted(d for d in dirs if d not in SKIPPED_DIRS) if recursive else []
            for name in sorted(files):
                if name.endswith(".ttl"):
                    (definitions if is_definitions_file(name) else trees).append(str(pathlib.Path(current) / name))
    except OSError:
        pass
    return sorted(definitions), sorted(trees)


def split_location(location: str) -> tuple:
    """``(path, kind, detail)`` of a location string (docs/cli.md, Locations): ``path`` -> (path, "file", ""); ``path#Heading``
    -> (path, "heading", Heading); ``path::Symbol`` -> (path, "symbol", Symbol)."""
    if "::" in location:
        path, _sep, symbol = location.partition("::")
        return path, "symbol", symbol
    if "#" in location:
        path, _sep, heading = location.partition("#")
        return path, "heading", heading
    return location, "file", ""
