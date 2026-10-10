---
name: anchor-document
version: 0.2.0
description: The intent document, the anchors.json record of truth and its rendered anchors.md, ID rules, provenance, versioning, promotion, and retirement.
---

# The anchor documents

Two documents land in `doc/plan/phase-current/` for people, and one machine record feeds everything else.

| File | Who edits | Purpose |
| --- | --- | --- |
| `intent.md` | you, from `assets/templates/intent.md` | One page: what the plan is for, what success looks like, what is out of scope. Frames the anchors; is never judged |
| `anchors.json` | `scripts/jev_judge.py` only (`add`, `update`, `retire`, `promote`, `set-beads`) | Record of truth. Every anchor with provenance, status, question overrides, fixtures, bead IDs, history |
| `anchors.md` | nobody; rendered by `render` | Human view of `anchors.json`. Re-render after every change |
| `anchor-runs/*.json` | `judge` (dry runs included; `check-fixtures` under `fixtures/`) | One file per gate run with the answers, attempts, bytes sent and a hash of the questions asked, so runs can be diffed and coverage computed |

Keep all of them in git with the plan. They are the plan's memory.

## anchors.json

```json
{
  "schema": "plan-anchoring/anchors/1",
  "plan": "phase-current",
  "intent_ref": "doc/plan/phase-current/intent.md",
  "anchors": [ { ...anchor record... } ]
}
```

Anchor record fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | string | `ANC-NNN` allocated by the script, or the register ID for REQ/ADR (`REQ-014`, `ADR-0007`) |
| `source_type` | `ANC` / `REQ` / `ADR` | Where the fact comes from |
| `source_ref` | string or null | The register ID for REQ/ADR; null for ANC (promotion records the ADR in `promoted_to`) |
| `statement` | string | The verbatim words. Immutable |
| `stated_by`, `stated_at`, `context` | string | Provenance: who, ISO date, "planning session" or register name. Quoted in every failing verdict |
| `summary` | string | Short label for tables; never judged |
| `scope` | string | Where the anchor applies; sent to Jev in `anchor.scope` for the relevance question |
| `deterministic_check` | string | Code-level check to pair with the development gate |
| `fixtures` | `{pass: [], fail: []}` | Example sentences that would satisfy or violate the anchor |
| `question_overrides` | `{plan?: {...}, dev?: {...}}` | Optional question wording overrides, same shape as a question-set file (`questions`, optional `roles`) |
| `version` | int | Bumped when scope, summary, fixtures, overrides, or status change. Never when the statement changes, because the statement never changes |
| `status` | `proposed` / `locked` / `promoted` / `retired` | Only `locked` and `promoted` anchors are judged by `--all` |
| `promoted_to` | string or null | The ADR an anchor was promoted to |
| `beads` | `{anchor, plan_questions, dev_questions}` | Bead IDs of the anchor tree |
| `history` | list | Append-only events with timestamps |

The full example is `assets/examples/anchors.example.json`. Run `validate` after any change; it fails on structural problems and warns when `intent_ref` points at a missing file or no anchor is locked. `judge` refuses a record with problems. `stated_at` defaults to today's local date.

## IDs

- `ANC-NNN` is allocated as max existing plus one, zero-padded to three digits. IDs are never reused, even after retirement.
- REQ and ADR anchors use their register ID as the anchor ID, so a report line reads `bd-42 · REQ-014 · not addressed` and points straight at the register.
- Bead IDs are recorded on the anchor, not the other way round, so the record stays the single place to look.

## Lifecycle

```
proposed  -->  locked  -->  promoted (keeps ID, gains ADR ref)
    \             \
     \             -->  retired (directive changed or withdrawn; new anchor replaces it)
      -->  retired (never confirmed)
```

- **proposed**: recorded so it survives compression; not judged.
- **locked**: judged at both gates. Requires the user's confirmation.
- **promoted**: `promote --id ANC-012 --adr ADR-0007`. The anchor keeps its ID and history; the ADR register gains the decision. Still judged.
- **retired**: `retire --id ANC-012 --replaced-by ANC-020 --reason "..."`. Not judged. The replacement ID and reason land in `history` so the chain is visible.

## Why the statement is immutable

Every past run quotes the statement it judged. If the statement could change, the history of verdicts would silently refer to different facts. A reworded directive is a different fact: new anchor, old one retired, both visible in `anchors.md`.

## Rendering

`render` writes `anchors.md` with a status table and one section per anchor: the quote as a blockquote, provenance line, summary, scope, deterministic check, bead IDs, and fixtures. Re-render after every `add`, `update`, `set-beads`, `promote` or `retire`, and commit both files together.
