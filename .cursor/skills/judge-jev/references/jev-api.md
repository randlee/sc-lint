---
name: jev-api
version: 0.1.0
description: How jev_judge.py talks to TypeSafe's Jev - endpoint, auth, request and response shape, validation, limits, the retry and context-shortening policy, every error code and flag, model pinning, and the committed allow_remote_judge setting that gates every call.
---

# Jev API as used by this skill

Jev is TypeSafe's decision model: it answers typed questions about a piece of state with probabilities and confidence, in parallel, and generates no text. `scripts/jev_judge.py` implements everything below; this document is for troubleshooting, not for hand-rolling requests.

## Remote judging consent (read first)

A live run sends record text off the machine: `quality` sends each record and its wave's questions, `batch` sends each request line. Sending it to TypeSafe is the user's decision, so the script fails closed:

- `TYPESAFE_API_KEY`, when present, must be well-formed (non-empty, ASCII, no control characters); when it is absent the script tries the documented platform credential store, and
- the repository's committed `.sc/judge-jev/settings.yaml` must set `allow_remote_judge: true`. `false` is the user's no; an absent resolved credential means the live call is refused. Both are refused.

The answer is recorded once per repository, never per session: ask the user, then `python3 scripts/judge_jev_settings.py set allow_remote_judge true|false` (or the installer's `--allow-remote-judge` / `--no-remote-judge`), and commit the file. Only the user decides it: agents never write it and never infer it from existing run files or past runs. The setting is found from `--repository-root` (default `.`); reading a present settings file needs PyYAML (`DEPENDENCY.PYYAML`, exit 2, otherwise), and nothing else in `jev_judge.py` does. `--dry-run` writes request previews after normal presentation redaction and `totals {requests, bytes}` to the run directory. Previews may differ from canonical request bodies when text is redacted. `--answers-file` runs the gating with fixture answers, offline. The key is never printed, logged, written to a run file, or included in an error.

## Endpoint and auth

- `POST https://api.typesafe.ai/v1/systemone`
- `Authorization: Bearer $TYPESAFE_API_KEY`, `Content-Type: application/json`

## Request

```json
{
  "model": "jev-latest",
  "state": {
    "anchor": {"id": "ANC-012", "statement": "...", "scope": "..."},
    "item": {"id": "ex-42", "title": "...", "text": "..."}
  },
  "questions": {
    "relevant":     {"type": "choice", "instructions": "...", "criteria": {"yes": "...", "no": "...", "unclear": "..."}},
    "addressed":    {"type": "choice", "instructions": "...", "criteria": {"yes": "...", "no": "...", "unclear": "..."}},
    "contradicted": {"type": "choice", "instructions": "...", "criteria": {"yes": "...", "no": "...", "unclear": "..."}}
  }
}
```

- `state` may be a string, object, or array; questions reference fields with backticked paths such as `item.text`.
- `questions` is keyed by your own IDs; the `instructions` are sent, the ID is not.
- Types: `choice` (criteria map option to description; up to 255 options), `score` (2 to 10 ordered level descriptions), `noul` (probability of yes, no confidence). This skill uses `choice` so every answer carries a confidence.

## Response

```json
{
  "model": "jev-1.13.0",
  "answers": {
    "relevant":     {"type": "choice", "choice": "yes", "probabilities": {"yes": 0.96, "no": 0.03, "unclear": 0.01}, "confidence": 0.94},
    "addressed":    {"type": "choice", "choice": "no",  "probabilities": {"yes": 0.08, "no": 0.88, "unclear": 0.04}, "confidence": 0.85},
    "contradicted": {"type": "choice", "choice": "no",  "probabilities": {"yes": 0.30, "no": 0.62, "unclear": 0.08}, "confidence": 0.55}
  },
  "usage": {"input_tokens": 410, "output_tokens": 9}
}
```

`probabilities` says which option; `confidence` says whether to trust the call. The script validates every response before gating: answer IDs equal the question IDs, each `choice` is a criteria option, `confidence` and every probability lie in [0, 1], and probabilities sum to 1 within rounding. Anything else is `JEV.RESPONSE_INVALID`, retried as transient. The response read is bounded at 1 MiB. The run file stores the full answers so thresholds can be re-tuned without re-judging.

## Size, context window, and shortening

- Jev allows about 32k tokens for state plus the longest question and about 64k for state plus all questions.

## Models

`jev-latest` moves with releases. Once thresholds are tuned, pin the version the response reported (`model_answered` in the run file) with `--model jev-1.13.0`.

## Error codes

| Code | Emitted by | Meaning and action |
| --- | --- | --- |
| `AUTH.CREDENTIALS` | pre-flight, 401 | Key missing, malformed, or rejected. Not retried |
| `POLICY.REMOTE_JUDGE_DISABLED` | pre-flight | `allow_remote_judge` is `false` or absent. Absent: ask the user once, record it with `judge_jev_settings.py set`, commit. `false`: only the user changes it. `--dry-run` works either way |
| `DEPENDENCY.PYYAML`, `SETTINGS.*`, `REPO.NOT_FOUND` | pre-flight | The settings file could not be read; fix what the error names. Never a silent default |
| `JEV.RATE_LIMITED`, `JEV.OVERLOADED`, `JEV.HTTP`, `JEV.NETWORK` | transport | Transient; retried 3 times, then the command exits 2 (the cell is `error` in the partial run) |
| `JEV.RESPONSE_INVALID` | validation | Malformed or oversized response; retried, then the command exits 2 (the cell is `error` in the partial run) |
| `JEV.CONTEXT_EXCEEDED` | 422 token limit (development gate) | Evidence could not be shortened enough; shorten it |
| `JEV.VALIDATION` | 422 other | Request rejected; usually a malformed override |
| `VALIDATION.ANCHORS`, `VALIDATION.INPUT` | record / arguments | Fix `anchors.json` (`validate`) or the flags |
| `QUESTIONS.ROLES` | question set | An override renamed a question ID; keep the default IDs |
| `INPUT.MISSING`, `INPUT.MALFORMED`, `INPUT.EMPTY` | files | A path is not a file, not JSON, or the item text is empty |
| `FIXTURE.MISSING`, `FIXTURE.NONE` | `--answers-file` | No fixture answer for an anchor; no anchor has fixtures |
| `SCRIPT.INTERNAL` | anywhere | Unexpected exception, reported as JSON with exit 2 |

## Exit codes

A tool failure is the only nonzero exit. Exit 0 means the command ran (`success: true`); read the outcome from `data`, never from the exit code. Exit 2 means it could not do its job (`success: false` with a typed `error`). The only other exit code is 130, an interrupt.

| Subcommand | Outcome field (exit 0) |
| --- | --- |
| `batch` | No envelope: stdout is the result lines. Exit 0 when every line has an answer; exit 2 with the error envelope on stderr when any line has `ok: false` |

Exit 2 covers bad arguments, missing or unreadable files, invalid settings, `POLICY.REMOTE_JUDGE_DISABLED`, credential failure, any Jev call left without an answer, a missing dependency, and `SCRIPT.INTERNAL`. Exit 130 (`CANCELED`, `recoverable: true`) is an interrupt.

## Batch runner

Two layers and a logged stream. Nothing in them writes to the tracker: reading the results, choosing a candidate and sealing it stay with the anchor-tuning agent.

| Subcommand | Input | Output |
| --- | --- | --- |
| `batch` (low layer, tracker-neutral) | JSONL request lines on stdin or `--input <file>` (one parser for both), each `{"item": {id, title, text}, "anchor": {id, statement, scope, kind}, "questions": [{candidate_id, questions, roles?}, ...]}` (`assets/schemas/jev-batch-request.schema.json`) | One JSONL result line per (item, candidate, repeat) on stdout, written as soon as that line and every line before it have an answer, never held for the end; each carries `request_id` (`<prefix>/<item>/<anchor>/<candidate>/r<n>`), `run_id`, `request_sha256`, `answers` with probabilities, `verdict` at `--threshold`, `leaning` at 0, `retry_count` and `attempts` (`assets/schemas/jev-batch-result.schema.json`) |

Every result line is also logged through the sc-observability logger (REQ-JEV-0008), flushed after every line, as three events: `request` (the request as sent), `answer` (Jev's raw response and attempts; none on a dry run) and `result` (the rest of the line); every event carries `request_id`, `run_id` (also its `correlation_id`) and `run` (`--run-name`, default a UTC timestamp). The log is `<run-dir>/logs/judge-jev.log.jsonl` (default run dir `<anchors dir>/anchor-runs`), appended to, and is read back with sc-observability's query API. A missing `sc-observability` package is `DEPENDENCY.SC_OBSERVABILITY` and a refused event `LOG.WRITE` (exit 2); run the script with `uv run --script`, whose inline metadata pins `sc-observability==1.5.0`. At most 4 calls run at once (`--max-workers`, 1 to 4). A failed call is retried once (recorded in `retry_count` and `attempts`); `AUTH.CREDENTIALS` and `POLICY.*` stop the remaining calls. Every request line is validated, and every request size checked against `--max-request-bytes` (at most 64000, since each request is logged whole and the logger refuses events over 64 KiB; a larger value is `VALIDATION.INPUT`), before the first call. A live run needs a key and `allow_remote_judge: true` (`--repository-root`); `--dry-run` and `--answers-file` (keys `<item>/<candidate>/r<n>`, `<item>/<candidate>`, `<candidate>`, `*`; a list gives successive attempts; `{"__error__": "<code>"}`, a code of at most 64 characters, simulates a failure) never contact Jev. The file's envelope is checked before the first request, as `INPUT.MALFORMED` (an object of string keys, each an object or a non-empty list of objects; an `__error__` object holds only a string code); an answer's own body is Jev's response stand-in and is checked after the send, as `JEV.RESPONSE_INVALID`.

## Record quality

`quality (--req|--adr|--nfr <id>)... --index <raptor index> [--questions <file>] [--dry-run]` judges whether each named record is well written (REQ-JEV-0101, REQ-JEV-0209); the record flags repeat and mix, and one invocation judges every named record (REQs, then ADRs, then NFRs, each in the order given) under one run name. The records come from a Raptor import index (the JSON Raptor's `scripts/extract.py --output` writes); every record is loaded and size-checked before the first call. A record with import diagnostics is refused with `RECORD.DIAGNOSTICS` and never sent, an unknown id is `RECORD.NOT_FOUND`. The decision tree is `assets/questions/quality-<kind>.json`, or the `--questions` file (for every record; a tree whose `kind` differs from the record's kind is refused with `QUALITY.TREE` before any call): a node is a wave or a leaf (`verdict`). A wave has `questions` and exactly one of `next`, an ordered list of `{when: {question id: choice}, goto: node}` (every pair matches; the first matching rule wins), or `verdict_rules`, an ordered list of `{any: {question id: choice}, verdict}` (matches when any listed question got its listed choice, or always without `any`; the first match is the verdict and the walk ends at that wave). The shipped trees hold wave 1 only, with `verdict_rules` `no` if any question answers `no`, else `unclear` if any answers `unclear`, else `yes`. Each wave is one `batch` request without an anchor, carrying only the record (as Raptor's JSON) and that wave's questions. The whole tree is checked when it loads, before any call and in `--dry-run` alike: every node, every question (a `choice` question with non-empty `instructions` and 2 to 255 `criteria`, option and description both non-empty text), every rule (each listed choice an option of its question, each `goto` a node), a last rule without conditions in each wave (so every combination of answers has an outcome), no cycle through `next`, and every later wave's statements and request size for the record. Any failure is `QUALITY.TREE` naming the wave, question or rule; a leaf holds only its `verdict`. A JSON input holding NaN, Infinity, an integer over 4300 digits, nesting too deep to parse or a lone surrogate escape is `INPUT.MALFORMED`. The run log must be writable before the first call, and a run whose log did not grow is `LOG.WRITE` (carrying the results). No input crashes the script. Output: `data.results`, one per record, each with `record`, `tree`, `verdict`, `path` (the node ids walked) and `waves`; each wave has `node`, `request_id`, `reason` (every question in tree order: `id`, `text`, `choice`, `confidence`), `next` and `verdict` (null unless the wave ended the walk); `data.log` names the run and the log root. Every result and log event carries `tree_id` and `tree_sha256` (the file's bytes). Each wave is logged as its `request`, `answer`, `result` and `wave` (request, answers, choices, reason, `next`, `verdict`) events, then one `verdict` event, all with the run name, in `<run-dir>/logs/judge-jev.log.jsonl` (default run dir `.sc/judge-jev`). A wave without an answer stops the run (exit 2) with the path so far and the completed `results`. `--dry-run` reports, per record, the tree, the record and its source, and the composed wave-1 request with its size against the 24000-byte limit, and calls nothing; `--answers-file` keys are `<record>/<node>/r1`, `<record>/<node>`, `<node>` or `*`.

## Definitions, `compare-definition` and `doctor`

A definition (ADR-JEV-0008) is data in the request state, never in a question: `state.definitions` is `[{term, text}]` sorted by term, and `state.location` names the location of a request made at one. `quality` and `compare-definition` take `--definitions <file>` (repeatable; default every `definitions.ttl` and `*.definitions.ttl` in `<--repository-root>/.sc/judge-jev/`). A definition attached to a record alone is in every request about it; one attached to a record and a location is only in a request at that location. A request no definition reaches is byte-identical to one without the feature. Definitions are never truncated and count toward `--max-request-bytes` and the log-event size check, before any call. Each file is Turtle checked by `assets/shapes/definitions.shacl.ttl` and loaded as its own named graph, so a tree's digest does not change. A bad file is `INPUT.MISSING`, `INPUT.MALFORMED` or `DEFINITIONS.INVALID` (over 128 KiB, a blank-node definition, a literal record, an attachment with two records, an over-long or multi-line term, an absolute or `..` path, a definition IRI in two files), never a crash. Every request, answer, result, wave and verdict event carries `definitions: [{iri, term, digest}]`; the digest is the SHA-256 of term, text and source.

`compare-definition (--req|--adr|--nfr <id>) --index <file> --definition <iri>... [--location <loc>] [--threshold <p>]` runs the record's walk without and with the candidates (all dropped from one arm, all kept in the other; the record's other definitions are in both) and reports each answer's choice and confidence on both sides. The candidates are `material` when an answer's choice flips or its confidence crosses `--threshold` (default 0.6, at or above decides); a move on one side of it is not material. `data.threshold` and the `compare` log event record the threshold applied. Arms are logged as `<run>.without` and `<run>.with`. `--dry-run` shows both composed requests; `--answers-file` keys `with/<record>/<node>`, `without/<record>/<node>` (or `with/*`, `without/*`) belong to one arm and win over the shared `<record>/<node>`, `<node>` and `*`. A candidate no request reaches is `DEFINITIONS.NOT_ATTACHED`, an unknown IRI `DEFINITIONS.NOT_FOUND`.

`doctor [<path>...] [--index <file>] [--repository-root <dir>]` checks the shipped trees, every tree and definitions file under `.sc/judge-jev/` and the argument paths, and calls nothing (no Jev, no key, no log, no file written). Errors, exit 2 with the report in `data`: a schema or model failure, a SHACL violation (edges, cycles, closed shapes), an unloadable definitions file, a definition IRI in two files, `QUESTIONS.DUPLICATE_IRI` (a question IRI declared in more than one tree file, naming every file) and `QUESTIONS.DUPLICATE_ID` (a `jj:id` used twice within one tree). Warnings, exit 0: `DEFINITION.UNATTACHED`, `DEFINITION.RECORD_UNKNOWN` (with `--index`), `DEFINITION.LOCATION_MISSING`, `DEFINITION.SOURCE_MISSING`. Notes: `DOCTOR.INDEX_SKIPPED` when no index is given, `DOCTOR.SYMBOL_UNCHECKED` for a `path::Symbol` location.

## Input models and `validate`

Every JSON input is parsed by `strict_loads`, then into a strict pydantic v2 model (the quality tree, the question set, the answers file and each Jev answer, the Raptor index, the batch request line). Unknown fields, wrong types and every cross-field rule are refused with the command's usual code (`QUALITY.TREE`, `VALIDATION.INPUT`, `QUESTIONS.ROLES`, `INPUT.MALFORMED`, `JEV.RESPONSE_INVALID`) and a message of the form `path: rule` (up to 5 problems), e.g. `nodes.wave-1.questions.context.criteria: Dictionary should have at least 2 items`. Raptor records stay open (extra fields are kept) but need a string `id`.

`validate (--tree|--questions|--answers|--index|--requests) <file>` (`-` reads stdin; exactly one kind) runs that model and the checks the other commands apply before their first request. `validate --answers` checks the answers-file envelope only (the check `--answers-file` makes before the first request), not the answers inside it. It never calls Jev, needs no API key and writes no log. On success `data` is `{kind, path, sha256}`; on failure the typed error carries the field path, with the same code `quality` or `batch --dry-run` would give. `--requests` also accepts `--gate` and `--max-request-bytes`. `validate --tree` does not check the parts that depend on a record (statements, request size, log events).

Invalid UTF-8 on stdin is `INPUT.MALFORMED`. Before the first call (and in `--dry-run`), the request event, the largest possible answer event and the result event of every request are measured the way the sc-observability logger measures them (compact JSON, UTF-8, 65536 bytes); one that would not fit is `JEV.CONTEXT_EXCEEDED` naming the event, never a failure after Jev has answered.

## Cost and latency

TypeSafe prices input tokens only (output is free) at a fraction of a cent per million tokens and quotes roughly 100 ms per request. A `quality` run is one request per wave per record, so judging 100 records costs effectively nothing; re-run after every edit. `--dry-run` reports `totals.bytes` to size a run without a key.

## Environment variables

| Variable | Default | Purpose |
| --- | --- | --- |
| `TYPESAFE_API_KEY` | none | Preferred TypeSafe credential for live runs; when absent, use the documented platform credential store |

It is the only environment variable the plugin reads. Model, threshold, size limits and near-threshold confirmation are the flags above, with checked-in defaults. The installed troubleshooting reference specifies the Keychain, Credential Manager, and Secret Service fallback entries.

Codex environment policy for `TYPESAFE_API_KEY`: `installation-and-troubleshooting.md`.
