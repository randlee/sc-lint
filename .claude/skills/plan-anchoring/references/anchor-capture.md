---
name: anchor-capture
version: 0.2.0
description: How the planning orchestrator detects, proposes, confirms, and words anchors while a plan is being discussed.
---

# Capturing anchors

An anchor is a fact the user stated while planning that roots the plan and keeps it from drifting. Capture happens in the main planning session because it is interactive; it is the one part of this skill that does not delegate.

## What counts as an anchor candidate

Listen for directives about how the work must be done, not what the work is. The plan already says what; anchors preserve the how and the why-not. Signals:

- A starting point: "copy the libraries from ATM as the starting point", "fork X", "reuse the existing parser".
- A prohibition: "don't add a new dependency for this", "no new crate", "never write to the DB from the CLI layer".
- A method or order: "migrate the schema first, then the API", "keep the trait object-safe".
- A boundary: "the CLI must stay thin; business logic lives in the library".
- A comparison to an existing system the user wants reproduced or avoided: "exactly what we have in ATM but general-purpose".
- A correction: when the user says "no, I said ..." the thing they said is an anchor candidate, because it has already drifted once.

Things that are not anchors: task lists, deliverables, estimates, or anything already numbered in the requirement or ADR register. Those are referenced as REQ or ADR anchors, not restated.

## Proposing and confirming

When you detect a candidate, propose it immediately and briefly, quoting the exact words:

> Anchor candidate: "copy the libraries from ATM as the starting point and remove the ATM-specific code". Lock it as ANC-012?

Record it only when the user confirms, or when they say "lock this", "anchor this", or an equivalent. If the user says "lock this" about something they said several turns ago, quote back what you believe they mean and confirm before recording. Until confirmed, a candidate may be recorded with `--status proposed` so it is not lost, and it is not judged at any gate until locked.

Do not batch candidates to the end of the conversation. The point of the skill is that facts survive context compression, and a proposal that waits is a proposal that gets compressed.

## Verbatim rules

The statement field is the anchor point. Paraphrase is exactly where meaning gets lost, so:

- Store the user's words as typed or transcribed, including informal grammar.
- Trim only leading and trailing filler ("so basically", "ok"). Mark any interior elision with `[...]`, and prefer not to elide at all.
- When a directive sits inside a long message, quote the minimal sentence or sentences that carry it, not the whole message.
- Never "clean up" the statement later. A changed directive is a new anchor; the old one is retired.
- The `summary` field is yours to write and is for tables only. Jev never sees it.

## Granularity

One testable constraint per anchor. "Copy the ATM crates and strip the ATM-specific code" is one constraint (the starting point and the cleanup are one act). "Copy the ATM crates and keep the CLI thin" is two: record two anchors, each quoting the sentence that carries its constraint, even if that means both quote part of the same message.

The test for granularity: can Jev answer "does this bead address the directive" with one yes or no? If a bead could address half of it, split.

## Fields you write at lock time

| Field | Purpose | How to write it |
| --- | --- | --- |
| `scope` | Feeds the relevance question so unrelated beads are `not_applicable` rather than failures | Name the area or kind of work the anchor governs: "the runtime crates sprint and anything that creates the core library implementation" |
| `summary` | Table label | Under 12 words |
| `deterministic_check` | The code-level proof the development gate should pair with | A diff, git log, grep, or test that would show the directive was literally followed |
| `fixtures.pass` / `fixtures.fail` | One sentence each that a bead would say if it satisfied or violated the anchor | Draft them, read them back, get the user's approval, then record with `update --fixture-pass/--fixture-fail`. `check-fixtures` sends them to Jev (same opt-in) to confirm the question wording lands them as expected |

Record with `add`; the script allocates the next ANC number so two sessions never collide on an ID. `scope`, `summary`, the deterministic check and fixtures can be set on `add` or later with `update`. Then `validate`, `render`, and create the bead tree per `bead-layout.md`.

## Re-opening a plan

When a planning session resumes, read `anchors.md` before reading the plan, and say so: "Loaded 14 anchors for phase-current." The user should never have to re-state a locked fact. If they do re-state one, check whether the existing anchor's wording matches; if it differs materially, propose a new anchor and retire the old.
