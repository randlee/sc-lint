---
id: B.4
title: QA Process Hardening
status: planned
branch: feature/phase-B-sprint-plans
worktree: <repo-worktree>/feature/phase-B-sprint-plans
target: develop
---

# Sprint B.4 — QA Process Hardening

## Goal

- codify the triage-first QA flow needed to reduce repeated sprint QA churn
- lock in bounded `rust-best-practices` usage so broad structural review
  happens on the first quality pass and later rounds stay in targeted fix mode
- make the prompt/process surface auditable enough that sprint closeout can run
  from the `qa-triage` agent definition, fix-assignment template, and
  regression-tested triage scripts rather than ad hoc routing decisions

## Hard Dependencies

- [docs/sc-lint/phase-B-plan.md](./phase-B-plan.md)
- `.claude/skills/triaging-findings/SKILL.md`
- `.claude/skills/todo-triage/SKILL.md`
- `.claude/skills/codex-orchestration/SKILL.md`
- `.claude/agents/quality-mgr.md`
- `.atm.toml`

## Exact Targets

- `.atm.toml`
- `.claude/agents/quality-mgr.md`
- `.claude/agents/qa-triage.md`
- `.claude/skills/team-lead/SKILL.md`
- `.claude/skills/codex-orchestration/SKILL.md`
- `.claude/skills/codex-orchestration/fix-assignment.xml.j2`
- `.claude/skills/triaging-findings/SKILL.md`
- `.claude/skills/todo-triage/SKILL.md`
- `scripts/find_todos.py`
- `scripts/triage_carry_forward.py`
- `scripts/test_find_todos.py`
- `scripts/test_triage_carry_forward.py`
- `docs/sc-lint/phase-B-plan.md`
- `docs/project-plan.md`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- the QA flow explicitly requires triage before fix dispatch for sprint QA
  failures
- `rust-best-practices` is required on QA-1 and does not re-run as a default
  broad reviewer on QA-2 and later fix rounds
- later QA rounds operate in targeted-fix mode with TODO scanning and
  carry-forward triage inputs available to the routing layer
- the prompt/process surface identifies the authoritative scripts, skills, and
  handoff points needed for the triage-first workflow
- `scripts/test_find_todos.py` is created as the regression test entrypoint
  for TODO discovery behavior
- `scripts/test_triage_carry_forward.py` is created as the regression test
  entrypoint for carry-forward triage classification behavior
- the triage-first workflow lands with explicit test coverage for TODO
  discovery and carry-forward classification so later prompt changes do not
  silently break routing behavior

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```text
QA-1:
- broad review set
- includes rust-best-practices

QA-2+:
- triage-first routing
- targeted fix verification
- carry forward unresolved QA-1 structural findings instead of re-running the
  same broad structural review by default
```

```json
{
  "round": "QA-2",
  "triage_records": [
    {
      "finding_id": "REQ-001",
      "classification": "structural",
      "carry_forward": true
    }
  ],
  "run_rust_best_practices": false
}
```

## This Sprint Does Not Close

- product-rule implementation
- architecture ADR acceptance
- Homebrew distribution planning

## Acceptance Criteria

- the plan explicitly states that QA failures go through triage before fix
  dispatch
- the plan explicitly states that `rust-best-practices` is required on QA-1 and
  is not the default broad reviewer on QA-2+
- the plan explicitly requires TODO scan and carry-forward triage inputs in the
  later-round routing flow
- the plan identifies the prompt, template, and script surfaces that own this
  process, including `qa-triage` and `fix-assignment.xml.j2`
- the sprint requires regression coverage from the newly created
  `scripts/test_find_todos.py` and `scripts/test_triage_carry_forward.py`
  files instead of relying on prompt prose only

## Required Validation

- `python3 -m unittest scripts/test_find_todos.py`
- `python3 -m unittest scripts/test_triage_carry_forward.py`
- `just lint`
