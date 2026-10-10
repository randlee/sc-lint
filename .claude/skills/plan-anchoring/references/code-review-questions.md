---
name: code-review-questions
version: 0.2.0
description: The development-gate (sprint review) Jev question set, the evidence question, what delivered-work text to feed it, and pairing with deterministic checks.
---

# Development-gate questions

The wording of record is `assets/questions/dev.json`. The development gate is run by the QA side of the dev/QA matrix (the quality-manager role) through the `anchor-sprint-review` agent, against the delivered work rather than the plan.

## What is being judged

Jev judges the text it is given. For the development gate that text is an account of what was built, assembled by the agent in this priority order and truncated at 24,000 characters from the bottom:

1. The sprint report or completion summary, if one exists.
2. The PR description (title and body).
3. `git diff --stat <base>..<head>` output, which lists the files touched.
4. Test summary lines (names and pass/fail counts, not full logs).
5. Titles and `close_reason` of the sprint's closed child beads, composed by `compose-item --children-json`.

Record what commands printed, never a paraphrase. The account is labelled by section so Jev can tell a claim ("we copied the ATM crates") from an artifact ("crates/runtime/src/lib.rs | 412 +-"). Full diffs are not sent: they blow the state budget and add noise.

## The four questions

| ID | Role | Asks | Isolates |
| --- | --- | --- | --- |
| `relevant` | relevance | Did the work touch the subject the anchor governs? | Scope; unrelated sprints are `not_applicable` |
| `implemented` | compliance | Does the account say the directive was followed in what was built? | Abandoned in code after being planned |
| `violated` | conflict | Does the account show the work did what the directive rules out? | Drift during implementation |
| `evidenced` | evidence | Where compliance is claimed, is checkable evidence cited (paths, diffs, commands, tests, comparisons)? | "Said it" vs "showed it" |

The first three mirror the planning gate so the same anchor is checked for the same failure modes at both points. The fourth exists because of the PRD's stated limitation: Jev confirms the work *describes* following the directive, not that the code did. A compliance claim with no checkable evidence becomes the `asserted_only` verdict, which fails the gate until evidence is added or the paired deterministic check is run.

## Pairing with deterministic checks

Each anchor may carry a `deterministic_check`: a diff, git log, grep, or test that proves the directive literally. The agent returns every `satisfied` or `asserted_only` anchor's check in `deterministic_checks`; the quality manager runs them and records the outcome alongside the Jev verdict. Examples:

| Anchor | Deterministic check |
| --- | --- |
| "copy the libraries from ATM as the starting point" | `git log --follow crates/runtime` shows ATM history, or `diff -r atm-core/crates/runtime crates/runtime` is mostly deletions |
| "no new dependency for this" | `git diff <base>..<head> -- Cargo.toml` adds no `[dependencies]` line |
| "the CLI stays thin" | `rg 'sqlx::' src/cli/` returns nothing |

Jev tells you where to look; the check tells you what is there.

## Wording rules

Same as the planning gate (`plan-anchor-questions.md`): one judgment per question, direct field references, explicit negations, situations not degrees, same IDs and options. For the development gate also:

- `implemented` must say that intent or deferral does not count as followed, or a sprint report that says "to be done next sprint" passes.
- `evidenced` must say that "no compliance claim at all" answers `no`, so an account that never mentions the anchor cannot be `evidenced: yes`.

## Reporting

A failing development-gate line reads:

> **sprint-2 · ANC-012 · violated** You said (planning session, Oct 9): "copy the libraries from ATM as the starting point and remove the ATM-specific code" The delivered work went against this anchor.

The anchor that was faithfully planned but abandoned in code passes the planning gate and fails here, which is why the gates are separate.
