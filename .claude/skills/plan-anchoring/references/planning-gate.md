---
name: planning-gate
version: 0.2.0
description: Running the planning gate end to end - what to pass, what Jev sees, the command and exit codes, and how to read the results for one bead and for the whole plan.
---

# Planning gate

Judges plan text. For each planning bead, every locked anchor is asked: relevant, addressed, contradicted. Run after every plan revision; cost is negligible and the point is to catch the compression failure before any code exists.

## Inputs

| Input | Source | Notes |
| --- | --- | --- |
| Planning beads | open children of the phase epic: `bd children <epic> --json` | One agent run per bead; set-aside sanity or QA beads if the plan has them |
| Anchors | `doc/plan/phase-current/anchors.json` | Only `locked` and `promoted` anchors are judged; `validate` must pass |
| Question overrides | each anchor's plan-questions bead | Fenced JSON in the bead description; `{}` means defaults |
| Script | `scripts/jev_judge.py` next to `SKILL.md`, as an absolute path | Passed to the agent as `script` |

## What Jev sees per request

Exactly one anchor (`id`, verbatim `statement`, `scope`) and one bead's composed text (`# Title`, `## Description`, `## Design`, `## Acceptance criteria`, `## Notes`, as present), plus the three questions. The bead text is whatever `bd show --json` holds, composed by `compose-item`; never a summary written by the agent.

## Run

Delegate per bead (Claude Code) or follow `../../agents/anchor-plan-review.md` inline (Codex, Cursor). The underlying command the agent runs once:

```bash
python3 "$S" judge --anchors "$A" --gate plan --all --item-id <bead> --item-title "<title>" \
  --item-file "$W/item.txt" [--overrides-file "$W/overrides.json"] --run-dir doc/plan/phase-current/anchor-runs
```

| Exit | Meaning |
| --- | --- |
| 0 | Every anchor `satisfied` or `not_applicable`, or a dry run |
| 1 | At least one `contradicted`, `not_addressed`, `underspecified` or `error` verdict; the run file was written |
| 2 | Fatal before judging: credentials, opt-in, malformed record or override, unknown anchor |

Then, across all beads:

```bash
python3 "$S" coverage --runs doc/plan/phase-current/anchor-runs --gate plan --format md
```

## Reading results

| Verdict | Jev decided | What settles it | Then |
| --- | --- | --- | --- |
| `contradicted` | The bead proposes what the directive rules out | Re-read the bead text that was sent (`$W/item.txt`) at the governed section | Change the approach, or ask the user if their directive has changed; then `retire` and lock a new anchor |
| `not_addressed` | The bead is silent on the directive | Same | Add the directive's specific content: starting point, constraint, step |
| `underspecified` | Confidence below threshold or `unclear` | `min_confidence` across beads: low on one bead means a vague bead, low on many means a vague anchor | Sharpen the bead, or the anchor's `scope` or question override |
| `satisfied` | Directive present | Nothing | Nothing |
| `not_applicable` | Anchor does not govern this bead | If it should have, the anchor's `scope` is too narrow | `update --scope` |
| `error` | Jev could not be asked after retries | `results[].error.code` | Fix the cause; re-run that bead |

The gate passes when every bead's latest run exits 0 and `coverage` lists no uncovered locked anchor. An anchor uncovered everywhere means its scope is wrong or the plan lacks the bead that should carry it.

Before editing on any failing verdict, re-read the sent text at the point the anchor governs. If Jev looks wrong, say so in the posted report with the reason and ask the user, rather than rewording the bead until Jev agrees. Three runs on the same bead with the same failing set is a hard stop.

Post each bead's report on the bead: `python3 "$S" report --run <run> --format md > "$W/report.md" && bd comment <bead> --file "$W/report.md"`. Compare revisions with `diff --prev <older run> --next <newer run>`.
