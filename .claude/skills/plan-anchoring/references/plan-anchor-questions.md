---
name: plan-anchor-questions
version: 0.2.0
description: The planning-gate Jev question set, what each question isolates, why the set is orthogonal, and how to word overrides.
---

# Planning-gate questions

The wording of record is `assets/questions/plan.json`. This document explains it; do not copy question text from here into beads, copy it from the JSON.

## The state Jev sees

One anchor and one planning bead, nothing else:

```json
{
  "anchor": {"id": "ANC-012", "statement": "<verbatim>", "scope": "<where it applies>"},
  "item":   {"id": "bd-42", "title": "<bead title>", "text": "<bead description, design, acceptance, notes>"}
}
```

Jev's accuracy degrades when unrelated material sits in the state, so the agent never sends several beads, the intent document, or other anchors. The script keeps each request under 24,000 bytes by shortening the item from the tail, and halves it again if Jev reports its context window exceeded; the run records `item.truncated`, `item_chars_sent` and `item_shortened` (`jev-api.md`).

## The three questions

| ID | Role | Asks | Isolates |
| --- | --- | --- | --- |
| `relevant` | relevance | Does the bead's work touch the subject the anchor talks about? | Scope. An anchor that does not govern this bead must not count as a failure |
| `addressed` | compliance | Does the bead explicitly carry the directive's specific content? | "Forgotten": the compression failure |
| `contradicted` | conflict | Does the bead propose something the directive rules out, or a different approach? | "Drifted": did something other than what was asked |

Each is a `choice` with `yes`, `no`, and `unclear` options, so every answer carries a probability distribution and a confidence. The `unclear` option exists because Jev's guidance is to give a way out when neither option fits; without it, an ambiguous bead forces a confident-looking wrong answer.

## Why the questions are orthogonal

Jev answers all questions in one request independently; none can see another's answer. So each question must isolate one failure mode, and the combination is done afterwards in code (`verdict-gating.md`). The design rules:

- `relevant` judges subject overlap only and explicitly says "do not judge whether the item obeys the anchor".
- `addressed` judges presence of the directive's specific content only and explicitly says "not whether the item conflicts".
- `contradicted` judges conflict only and explicitly says "silence is not a contradiction".

If `addressed` and `contradicted` were one question ("does the bead comply?"), a forgotten anchor and a drifted anchor would get the same verdict and the planning agent would not know whether to add a sentence or rewrite the approach.

## Wording rules for overrides

Jev answers the question you wrote, not the one you meant. When overriding wording for a specific anchor in its plan-questions bead:

- Keep one judgment per question. Do not fold two checks into one.
- Point at fields directly with backticks: `item.text`, `anchor.statement`. Avoid "the former" or "a property of a property".
- State scoping words, negations, and implied conditions explicitly. No double negatives.
- Keep `instructions` and `criteria` consistent with each other; contradictions degrade answers.
- Describe situations in the criteria, not degrees ("the item names ATM's crates as the source" rather than "mostly addresses").
- Keep the same question IDs and the same `yes`/`no`/`unclear` options so the gating code still applies.
- Never ask Jev to count, compare numbers, check dates, or look across beads.

A good reason to override: an anchor whose directive is easy to satisfy in spirit but not in letter. For "copy the ATM crates as the starting point", override `addressed` to name the concrete content: "Does `item.text` state that the runtime crates are copied from ATM rather than written new?"

## Example request (dry run)

```bash
python3 "$S" judge --anchors "$A" --gate plan --anchor ANC-012 \
  --item-id bd-42 --item-title "Sprint 2 runtime" --item-file "$W/item.txt" --dry-run
```

The run file's `requests[0]` is the exact body that would be sent and `totals.bytes` its size. Review it the first time a new anchor is judged, and run `check-fixtures --gate plan --id ANC-012` after changing wording.
