---
id: G.0
title: Abandon Phase F And Record ADR-015
status: planned
branch: sprint/G.0-abandon-phase-F
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.0-abandon-phase-F
stack: A
stack_base: feature/phase-G-planning
target: develop (via stack A, PR base feature/phase-G-planning)
owner: clint
---

# Sprint G.0 — Abandon Phase F And Record ADR-015

## Goal

- close Phase F without merging any of it and record the replacement design
  decision so no later sprint can reintroduce a consumer-specific engine

## Hard Dependencies

- [phase-G-plan.md](./phase-G-plan.md)
- [docs/sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md](../../sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md)
- `integrate/phase-F` and `sprint/F.*` branches (read-only reference)

## Exact Targets

- `docs/sc-lint/adr/ADR-015-standard-repo-tools-adoption-kit.md` (new)
- `docs/sc-lint/adr/README.md`
- `docs/project-plan.md`
- `docs/phase-E/phase-E-plan.md` (status only)
- `docs/plans/phase-G/*` (this plan set, merged to develop)

## Deliverables

- ADR-015 with Status `Accepted`, recording: the six locked principles from
  the phase plan; that ADR-014 is rejected and never merged; the kit form
  (`packages/sc-lint-adoption` → consumer `plugins/sc-lint`); the sc-publish
  delegation rule for sc-lint setup.
- `docs/project-plan.md` links Phase G and marks Phase F abandoned with a
  one-line reason and the archive tag name.
- `docs/phase-E/phase-E-plan.md` frontmatter status changed to `implemented`
  (PR #104 merged).
- Git housekeeping performed by team-lead, recorded in the PR description:
  PR #128 closed unmerged; tag `archive/phase-F` at `integrate/phase-F` head;
  worktrees `sprint/F.*` and `integrate/phase-F` removed; branches deleted
  after tagging; `worktree-tracking.md` updated.

## Acceptance Criteria

- `ls docs/sc-lint/adr/ADR-015-*.md` exists and its Status table row is
  `Accepted`.
- `grep -c "phase-F" docs/project-plan.md` ≥ 1 and the line contains
  `abandoned`.
- `git tag -l archive/phase-F` prints the tag; `git worktree list` shows no
  `phase-F` or `sprint/F.` path.
- No `docs/sc-lint/adr/ADR-014*` file exists on `develop`.

## Required Validation

- `cargo test --workspace` unchanged (docs-only sprint).
- Docs link check: every relative link in `docs/plans/phase-G/*.md` resolves.

## Out Of Scope

- any code recovery (G.1)
- any change under `crates/`, `packages/`, `scripts/`
