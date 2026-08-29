---
name: plan-hardening
version: 1.5.0
description: >
  Team-lead drives plan hardening after the current plan state already exists
  in repo docs.
depends_on:
  codex-orchestration: 0.x
---

# Plan Hardening

Audience: `team-lead` only.

Use this only for phase-plan hardening before implementation starts or resumes.

## Assumptions

- the current plan state already exists in repo docs, though sprint docs may
  still be partial or missing
- do not ask the user to explain detailed plan content; read the planning docs
  and references directly after they are created
- `team-lead` routes the process but is not the authority for rewriting the
  plan
- the user-discussed deliverable scope is authoritative
- if no target phase worktree exists, create one from `develop` before
  starting

## Expected Result

Sprint plan approved by:
- `plan-scope-reviewer`
- `critical-plan-reviewer`
- `quality-mgr`

## Required Reference

Always use:
- `.claude/skills/plan-hardening/sprint-planning-guidelines.md`
- `.claude/skills/plan-hardening/references/installation-and-troubleshooting.md`

## Step 0 — Verify gh stack Installation

```bash
which gh && gh --version && gh stack --version
```

If not found on PATH, also check common install locations — Claude Code's
bash environment may not share PATH with the interactive shell:

```bash
for p in "/opt/homebrew/bin/gh" "$HOME/.local/bin/gh"; do
  [ -x "$p" ] && echo "Found at: $p" && break
done
```

If `gh` or the `gh-stack` extension is missing: **read
`references/installation-and-troubleshooting.md` before proceeding.** Do not
continue with degraded behavior; the stack protocol in every phase plan
depends on it.

## Execution Table

| # | Route to | Input required | Output expected | Read before executing |
|---|----------|----------------|-----------------|-----------------------|
| 1 | `clint` | vars file | `step-1` fenced JSON | `steps/step-1.md` |
| 2 | `plan-scope-reviewer` (background) | context + `step-1` JSON | `step-2` fenced JSON | `steps/step-2.md` |
| 3 | `clint` | `step-2` JSON | `step-3` fenced JSON | `steps/step-3.md` |
| 4 | `critical-plan-reviewer` (background) | context + `step-3` JSON | `step-4` fenced JSON | `steps/step-4.md` |
| 5 | `clint` | `step-4` JSON | `step-5` fenced JSON | `steps/step-5.md` |
| 6 | `quality-mgr` | `step-5` JSON + QA vars file | codex-orchestration plan-QA handoff | `steps/step-6.md` |

## Round Tracking

`team-lead` must keep a round table for every `/plan-hardening` run.

Minimum columns:

| Round | Step | Reviewer | reviewed_commit | status | blocking | important | minor | findings_hash | supersedes | Note |
|-------|------|----------|-----------------|--------|----------|-----------|-------|---------------|------------|------|

Use the example in:
- `.claude/skills/plan-hardening/examples/plan-hardening-rounds.example.md`

## Hard Stops

- `team-lead` only checks the top-level `status` and expected `mode` fields on
  each fenced JSON response before advancing
- every step after step 1 must receive the previous step's fenced JSON
- missing or malformed fenced JSON is a hard stop
- a reviewer rerun is valid only when either `reviewed_commit` changed or
  `findings_hash` changed
- if the same reviewer returns the same `reviewed_commit` and the same
  `findings_hash` again, treat it as a stale replay and do not open a new
  hardening round
- substantial scope drift from the user-discussed plan is a hard stop
- an ADR the phase depends on that is not `Accepted` on the planning branch
  before step 6 is a hard stop; ADRs and requirements updates land during
  planning, never inside a sprint
- remaining in-scope work without sprint ownership is a hard stop
- a phase plan without a `## Branch Stacks And Parallelism` section is a
  hard stop; sprints are planned as `gh stack` layers
- a plan whose sprints are serialized on CI pass or merge of the previous
  sprint, when they could start on commit, is a hard stop
- a sprint with a layer above it and no `## Unblock Milestone` section is a
  hard stop
- if a sprint cannot credibly land its committed deliverables at a
  production-ready level, split it before implementation
- if a reviewer loop returns `FAIL` three times without converging, escalate to
  the user before continuing

## Render

- `.claude/skills/plan-hardening/01-plan-scope-review.xml.j2`
- `.claude/skills/plan-hardening/02-sprint-scope-hardening.xml.j2`
- `.claude/skills/plan-hardening/03-consistency-hardening.xml.j2`
- `.claude/skills/plan-hardening/steps/step-1.md`
- `.claude/skills/plan-hardening/steps/step-2.md`
- `.claude/skills/plan-hardening/steps/step-3.md`
- `.claude/skills/plan-hardening/steps/step-4.md`
- `.claude/skills/plan-hardening/steps/step-5.md`
- `.claude/skills/plan-hardening/steps/step-6.md`
- `.claude/skills/plan-hardening/examples/plan-hardening-vars.example.json`
- `.claude/skills/plan-hardening/examples/plan-hardening-rounds.example.md`
- `.claude/skills/plan-hardening/examples/plan-hardening-qa-vars.example.json`
- `.claude/skills/plan-hardening/sprint-planning-guidelines.md`
- `.claude/skills/plan-hardening/references/installation-and-troubleshooting.md`
