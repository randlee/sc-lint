---
id: C.2
title: Published Interface Artifact Pipeline
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.2 — Published Interface Artifact Pipeline

## Goal

- define how interface reports are published for people and tools from one
  shared structured source
- ensure the report model follows the XHTML fragment/report pattern
- prevent drift toward hand-authored HTML documentation

## Hard Dependencies

- [docs/sc-lint/sprint-C1.md](./sprint-C1.md)
- [docs/sc-lint/version-requirements.md](./version-requirements.md)
- `/Users/randlee/.claude/skills/html-report/SKILL.md`

## Exact Targets

- `docs/sc-lint/phase-C-plan.md`
- `docs/sc-lint/sprint-C2.md`
- `docs/architecture.md`
- `docs/project-plan.md`

## Deliverables

- a planned report package model that includes:
  - main HTML report for human readers
  - JSON sidecar as the machine source of truth
  - optional XHTML fragments for section-level context
- coverage planning for published interfaces across:
  - all shipped crate public APIs
  - stable top-level CLI commands and machine contracts
  - RPC/socket interfaces when present
- explicit planning language that generated templates and structured data own
  the output, not manually maintained HTML pages

## Explicit Code Samples

```json
{
  "output_path": "artifacts/interfaces/sc-lint-cli/index.html",
  "json_output_path": "artifacts/interfaces/sc-lint-cli/index.json",
  "title": "sc-lint CLI Interface Report",
  "sections": []
}
```

```text
artifacts/interfaces/
  crate-api/
  cli/
  transport/
```

## This Sprint Does Not Close

- the semver decision logic for Rust APIs
- hard-fail policy enforcement in CI
- final implementation of HTML templates or rendering commands
- consumer-onboarding skill or marketplace delivery

## Acceptance Criteria

- the plan requires generated report packages rather than hand-written HTML
- the plan requires a JSON sidecar as canonical machine-readable output
- the plan requires optional XHTML fragments for deeper section-level context
- the plan requires published coverage for all shipped crates, not only the
  top-level crate

## Required Validation

- `just lint`
