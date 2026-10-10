---
name: development-gate
version: 0.2.0
description: Running the development gate (sprint review) end to end - what evidence to pass, what Jev sees, the command and exit codes, pairing with deterministic checks, and how to read the results.
---

# Development gate

Judges delivered work. The same anchors as the planning gate, asked of an account of what was built: relevant, implemented, violated, and evidenced. Run by the quality manager when a sprint is handed over, before acceptance.

## Inputs

| Input | Source | Notes |
| --- | --- | --- |
| Sprint | the sprint's epic bead ID | Item ID for the run; its closed children are listed by the script |
| Evidence | sprint report file, PR number or body file, test summary file | In that priority order; the agent stops at 24,000 characters |
| Anchors | `doc/plan/phase-current/anchors.json` | `locked` and `promoted` only |
| Question overrides | each anchor's dev-questions bead | Fenced JSON; `{}` means defaults |
| Script | `scripts/jev_judge.py` next to `SKILL.md`, absolute | Passed as `script` |

## What Jev sees per request

One anchor and the account: the sprint bead's title and fields, `## Closed beads` (id, title, `close_reason` per closed child, composed by the script), then `## Sprint report`, `## Pull request`, `## Files changed` (names and counts from `git diff --stat`), `## Tests` (summary lines). Record what commands printed; never a paraphrase. No full diffs, no source files, no logs.

## Run

Delegate to `anchor-sprint-review` (Claude Code) or follow `../../agents/anchor-sprint-review.md` inline. The command the agent runs once:

```bash
python3 "$S" judge --anchors "$A" --gate dev --all --item-id <sprint epic> --item-title "<sprint title>" \
  --item-file "$W/item.txt" [--overrides-file "$W/overrides.json"] --run-dir doc/plan/phase-current/anchor-runs
```

Exit codes as for the planning gate: 0 passed or dry run, 1 failing or `error` verdicts with the run file written, 2 fatal.

## Pairing with deterministic checks

The agent returns `deterministic_checks` for every `satisfied` or `asserted_only` anchor that carries one. Run each, record the outcome next to the Jev verdict in the posted report. Jev confirms the account *describes* following the directive; the check confirms the code did.

| Anchor | Check |
| --- | --- |
| "copy the libraries from ATM as the starting point" | `git log --follow crates/runtime` shows ATM history, or `diff -r atm-core/crates/runtime crates/runtime` is mostly deletions |
| "no new dependency for this" | `git diff <base>..<head> -- Cargo.toml` adds no `[dependencies]` line |
| "the CLI stays thin" | `rg 'sqlx::' src/cli/` returns nothing |

Set a check at lock time with `add --deterministic-check` or later with `update --deterministic-check`.

## Reading results

| Verdict | Jev decided | What settles it | Then |
| --- | --- | --- | --- |
| `contradicted` | The work did what the directive rules out | The deterministic check, or the diff-stat lines for the governed area | Block acceptance; the work changes or the user retires the anchor explicitly |
| `not_addressed` | The account does not show the directive was followed | The sprint report at the governed area | Block until the report shows it; re-run |
| `asserted_only` | Compliance claimed without anything checkable | Run the paired check | Pass once the check is recorded, or ask for evidence (paths, diff, command, test) |
| `underspecified` | Confidence below threshold | Usually a vague sprint report | Ask for a fuller report; re-run |
| `satisfied` | Followed and evidenced | Run the paired check anyway | Record it |
| `not_applicable` | The sprint did not touch the governed area | Expected for most anchors on most sprints | Nothing |
| `error` | Jev could not be asked after retries | `results[].error.code` | Fix the cause; re-run |

The gate passes when the sprint's latest run exits 0 and every returned deterministic check has been run and recorded. An anchor that was faithfully planned and then abandoned in code passes the planning gate and fails here; that is why the gates are separate.

Post the report on the sprint bead and, when there is a PR, on the PR. Compare with the previous handover using `diff`.
