---
name: jev-api
version: 0.2.0
description: How jev_judge.py talks to TypeSafe's Jev - endpoint, auth, request and response shape, validation, limits, the retry and context-shortening policy, every error code and flag, model pinning, and the privacy opt-in that gates every call.
---

# Jev API as used by this skill

Jev is TypeSafe's decision model: it answers typed questions about a piece of state with probabilities and confidence, in parallel, and does not generate text. That is exactly the adversarial-reviewer role this skill needs. Everything below is implemented in `scripts/jev_judge.py`; this document is for understanding and troubleshooting, not for hand-rolling requests.

## Privacy opt-in (read first)

A gate run sends two things off the machine: the anchor statement (the user's words) and the item text (a plan bead or an account of delivered work). Whether proprietary plans may be sent to TypeSafe is an open question in the PRD, so the script fails closed:

- `TYPESAFE_API_KEY` must be set and well-formed (non-empty, ASCII, no control characters), and
- `PLAN_ANCHOR_ALLOW_REMOTE_JUDGE=1` must be set for the session (or `--allow-remote` passed).

Without both, `judge` and `check-fixtures` return `AUTH.CREDENTIALS` or `POLICY.REMOTE_JUDGE_DISABLED` and send nothing. `--dry-run` writes the exact request bodies and `totals {requests, bytes}` to the run directory for review. `--answers-file` runs the gating logic with fixture answers and never touches the network. The key is never printed, logged, or written to a run file; error messages never include it. Set the opt-in per plan, not globally, until retention is settled.

## Endpoint and auth

- `POST https://api.typesafe.ai/v1/systemone` (override with `PLAN_ANCHOR_JEV_URL` for a proxy or mock)
- `Authorization: Bearer $TYPESAFE_API_KEY`, `Content-Type: application/json`

## Request

```json
{
  "model": "jev-latest",
  "state": {
    "anchor": {"id": "ANC-012", "statement": "...", "scope": "..."},
    "item": {"id": "bd-42", "title": "...", "text": "..."}
  },
  "questions": {
    "relevant":     {"type": "choice", "instructions": "...", "criteria": {"yes": "...", "no": "...", "unclear": "..."}},
    "addressed":    {"type": "choice", "instructions": "...", "criteria": {"yes": "...", "no": "...", "unclear": "..."}},
    "contradicted": {"type": "choice", "instructions": "...", "criteria": {"yes": "...", "no": "...", "unclear": "..."}}
  }
}
```

- `state` may be a string, object, or array; questions reference fields with backticked paths such as `item.text`.
- `questions` is keyed by your own IDs; the ID is not sent to the model, the `instructions` are.
- Types: `choice` (criteria is a map of option to description; up to 255 options), `score` (ordered list of 2 to 10 level descriptions), `noul` (probability of yes, no separate confidence). This skill uses `choice` so every answer carries a confidence.

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

`probabilities` says which option; `confidence` says whether to trust the call at all. The script validates every response before gating: answer IDs must equal the question IDs, each `choice` must be one of the criteria options, `confidence` and every probability must be in [0, 1], and probabilities must sum to 1 within rounding. Anything else is `JEV.RESPONSE_INVALID`, retried as a transient failure. The response read is bounded at 1 MiB. The run file stores the full answers so thresholds can be re-tuned later without re-judging.

## Size, context window, and shortening

- Jev allows about 32k tokens for state plus the longest question and about 64k for state plus all questions.
- Before the first call the script serialises the full body and shortens the item text (keeping the head, appending a truncation marker) until the request is under `--max-request-bytes` (default 24,000 bytes).
- If Jev still answers with a 4xx whose body reports the token limit, the script halves the item and retries, down to `--min-item-chars` (default 2,000). Below that the anchor is recorded as `verdict: error` with `JEV.CONTEXT_EXCEEDED` and the advice to shorten the evidence.
- Each result records `item_chars_sent`, `item_shortened`, `request_bytes`; the run's `summary.shortened` lists affected anchors.

## Attempt policy

Per anchor, per request:

| Situation | Behaviour |
| --- | --- |
| 429, 529, any 5xx, timeout, network error, invalid response | Up to 3 attempts. Delay before the second: `Retry-After` if present (capped at 30 s), else 1 s; before the third: 2 s. After the third failure the anchor is `verdict: error` with the last code; other anchors continue |
| 422 token limit | Shorten and retry as above (counts toward the 3 attempts) |
| 422 other | Not retried; `JEV.VALIDATION` with up to 300 characters of the body, which names the bad field |
| 401 | Not retried; the whole run aborts with `AUTH.CREDENTIALS` (body withheld) |
| Missing opt-in or key | Checked before any request; the run aborts |

Agents and the skill never add a retry loop around the script. `attempts` is recorded on every result. Questions in one request are answered independently and in parallel, so the three or four questions cost one round trip.

## Models

`jev-latest` moves with releases. Once thresholds are tuned for a plan, pin the versioned model the response reported (`model_answered` in the run file) via `PLAN_ANCHOR_JEV_MODEL=jev-1.13.0` or `--model`.

## Error codes

| Code | Emitted by | Meaning and action |
| --- | --- | --- |
| `AUTH.CREDENTIALS` | pre-flight, 401 | Key missing, malformed, or rejected. Not retried |
| `POLICY.REMOTE_JUDGE_DISABLED` | pre-flight | Opt-in not set. Use `--dry-run` or set `PLAN_ANCHOR_ALLOW_REMOTE_JUDGE=1` |
| `JEV.RATE_LIMITED`, `JEV.OVERLOADED`, `JEV.HTTP`, `JEV.NETWORK` | transport | Transient; retried 3 times, then per-anchor `error` |
| `JEV.RESPONSE_INVALID` | validation | Malformed or oversized response; retried, then per-anchor `error` |
| `JEV.CONTEXT_EXCEEDED` | 422 token limit | Item could not be shortened enough; shorten the evidence |
| `JEV.VALIDATION` | 422 other | Request rejected; usually a malformed override |
| `VALIDATION.ANCHORS`, `VALIDATION.INPUT` | record / arguments | Fix `anchors.json` (`validate`) or the flags |
| `QUESTIONS.ROLES` | question set | An override renamed a question ID; keep the default IDs |
| `ANCHOR.UNKNOWN`, `ANCHOR.DUPLICATE`, `ANCHOR.NONE`, `ANCHOR.NOT_LOCKED` | record | Wrong ID; already recorded; nothing locked; `--anchor` named a proposed or retired anchor (use `--include-unlocked` only for fixtures) |
| `INPUT.MISSING`, `INPUT.MALFORMED`, `INPUT.EMPTY` | files | A path is not a file, not JSON, or the item text is empty |
| `FIXTURE.MISSING`, `FIXTURE.NONE` | `--answers-file`, `check-fixtures` | No fixture answer for an anchor; no anchor has fixtures |
| `RUNS.NONE`, `RUNS.MISMATCH` | `coverage`, `diff` | No run files; runs for different gates or items |
| `SCRIPT.INTERNAL` | anywhere | Unexpected exception, reported as JSON with exit 2 |

Fatal codes abort with exit 2 and `success: false`. Transient codes end up in `results[].error` inside a successful run that exits 1.

## Exit codes

| Code | `judge` | `check-fixtures` | `validate` |
| --- | --- | --- | --- |
| 0 | gate passed, or dry run | every fixture landed as expected | no problems |
| 1 | a failing or `error` verdict; run file written | a mismatch | problems listed |
| 2 | fatal `JudgeError` or `SCRIPT.INTERNAL` | same | same |

## Judging flags

| Flag | Default | Purpose |
| --- | --- | --- |
| `--model` | `jev-latest` | Model alias or pinned version |
| `--threshold` | 0.6 | Confidence gate (see `verdict-gating.md`) |
| `--questions` | `assets/questions/<gate>.json` | Alternative question-set file |
| `--overrides-file` | none | Per-anchor overrides read from the question beads |
| `--max-item-chars` | 24,000 | Hard cap on item characters before composing the request |
| `--max-request-bytes` | 24,000 | Pre-flight cap on the serialised request |
| `--min-item-chars` | 2,000 | Floor for context shortening |
| `--timeout` | 30 s | Per-request socket timeout |
| `--run-dir` / `--out` | `<anchors dir>/anchor-runs` | Where the run file goes |
| `--dry-run`, `--answers-file`, `--allow-remote`, `--include-unlocked` | | See above |

## Cost and latency

TypeSafe prices input tokens only (output is free) at a fraction of a cent per million tokens, and quotes roughly 100 ms per request. A planning gate over 15 beads and 20 anchors is 300 requests and costs effectively nothing; re-running after every plan edit is the intended usage. `--dry-run` reports `totals.bytes` so a run can be sized without a key.

## Environment variables

| Variable | Default | Purpose |
| --- | --- | --- |
| `TYPESAFE_API_KEY` | none | Required for live runs |
| `PLAN_ANCHOR_ALLOW_REMOTE_JUDGE` | unset | Must be `1` for live runs |
| `PLAN_ANCHOR_JEV_MODEL` | `jev-latest` | Model alias or pinned version |
| `PLAN_ANCHOR_JEV_URL` | TypeSafe endpoint | Proxy or mock server |
| `PLAN_ANCHOR_CONFIDENCE_THRESHOLD` | `0.6` | Gating threshold |
| `PLAN_ANCHOR_MAX_ITEM_CHARS` | `24000` | Item truncation |

Under Codex, `TYPESAFE_API_KEY` matches its default `*KEY*` exclusion pattern for command environments: if you restrict `[shell_environment_policy]` in `config.toml`, keep `ignore_default_excludes = true`, and with an allowlist under `[shell_environment_policy.filters]` add `"TYPESAFE_API_KEY" = "include"`.
