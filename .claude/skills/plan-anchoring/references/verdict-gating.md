---
name: verdict-gating
version: 0.2.0
description: How jev_judge.py turns independent Jev answers into one verdict per anchor and item, the role contract, thresholds, severity order, pass criteria and exit codes, coverage, run diffs, and offline fixtures.
---

# Verdict gating

Jev cannot gate one question on another inside a request, and it cannot count. Both happen in `scripts/jev_judge.py`, so they are deterministic and testable with fixtures.

## The role contract

A question set maps roles to question IDs:

```json
"roles": {"relevance": "relevant", "compliance": "addressed", "conflict": "contradicted"}
```

`relevance`, `compliance`, and `conflict` are required; `evidence` is optional and used by the development gate. Overrides change wording only; do not rename question IDs or options. The script re-checks roles after merging every override and refuses a set whose roles do not resolve (`QUESTIONS.ROLES`) before any request leaves the machine.

## From answers to a verdict

Each `choice` answer gives a `choice` and a `confidence` in [0, 1]. The threshold `t` defaults to 0.6 (`--threshold`, or `PLAN_ANCHOR_CONFIDENCE_THRESHOLD`). Evaluated in order; the first match wins:

| # | Condition | Verdict |
| --- | --- | --- |
| 1 | relevance is `no` with confidence >= t | `not_applicable` |
| 2 | conflict is `yes` with confidence >= t | `contradicted` |
| 3 | relevance or compliance is `unclear`, or either confidence < t | `underspecified` |
| 4 | compliance is `no` | `not_addressed` |
| 5 | compliance is `yes` and an evidence role exists and evidence is `no` with confidence >= t | `asserted_only` |
| 6 | compliance is `yes` | `satisfied` |
| 7 | anything else | `underspecified` |

Contradiction outranks underspecification because a confident conflict is actionable even when relevance was shaky. A `noul` answer, if a set uses one, is mapped to yes/no at 0.5 with confidence `abs(p - 0.5) * 2`. Responses that fail validation never reach this table; they are retried and then recorded as `verdict: error`.

`min_confidence` on each result is the lowest confidence across the three required roles; it is reported so an `underspecified` verdict can be read as "the anchor needs sharper wording" when it recurs across beads.

## Severity order

`contradicted` > `not_addressed` > `asserted_only` > `underspecified` > `satisfied` > `not_applicable`. `error` sorts first in reports and ranks below every verdict in diffs, so losing a verdict is a regression. Results in a run are sorted by severity then anchor ID.

## Pass criteria and exit codes

- A **run** (one item) passes when no result is `contradicted`, `not_addressed`, `asserted_only`, `underspecified`, or `error`. `judge` exits 0 on pass or dry run, 1 on a failing run (the run file is still written), 2 on a fatal error before judging.
- The **planning gate** passes when every planning bead's latest run passes and `coverage` lists no uncovered locked anchor.
- The **development gate** passes when the sprint's latest run passes and each returned deterministic check has been run and recorded.

## Coverage, in code

```bash
python3 "$S" coverage --runs doc/plan/phase-current/anchor-runs --gate plan --format md
```

Takes the latest non-dry run per (gate, item), builds the anchor x item matrix, and lists anchors with no `satisfied` cell. Without `--gate`, columns are labelled `gate:item` so plan and dev verdicts for the same item are not merged. "Is ANC-012 satisfied anywhere?" is answered here, never by Jev. An anchor that is `not_applicable` to every bead is reported as uncovered on purpose: either its scope is wrong or the plan lacks the bead that should carry it.

## Diffing runs

```bash
python3 "$S" diff --prev anchor-runs/plan-<older>-bd-42.json --next anchor-runs/plan-<newer>-bd-42.json
```

Lists changed, added, and removed verdicts for the same gate and item (`RUNS.MISMATCH` otherwise), flags regressions, and shows pass before and after. Run it after every plan edit so the planning agent sees exactly what its change did. Two runs of the same item in the same second get distinct file names (`-2`, `-3`).

## Fixtures and offline runs

Two different things are called fixtures:

- **Fixture sentences** on an anchor (`fixtures.pass`, `fixtures.fail`, set with `update --fixture-pass/--fixture-fail`): what a bead would say if it satisfied or violated the anchor. `check-fixtures --gate plan --all` sends each sentence to Jev as the item (same key and opt-in as `judge`) and reports whether it landed as expected: pass -> `satisfied`, fail -> `not_addressed` or `contradicted`. Exit 1 on any mismatch. Use it after writing or overriding question wording.
- **Fixture answers** (`--answers-file`): Jev-shaped answers keyed by anchor ID (or `*`), injected instead of calling Jev. They exercise the gating table, report format, coverage and diffs with no key and no network; `assets/examples/answers.fixture.json` is one, and `tests/test_jev_judge.py` uses them. For `check-fixtures`, keys are `<anchor>:pass:<n>` / `<anchor>:fail:<n>`.

`--dry-run` writes the exact requests, plus `totals {requests, bytes}`, to the run directory and no verdicts. Dry runs are never counted by `coverage`.
