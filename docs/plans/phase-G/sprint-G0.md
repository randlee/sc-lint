---
id: G.0
title: Archive Rejected Phase F
status: planned
branch: sprint/G.0-abandon-phase-F
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.0-abandon-phase-F
stack: A
stack_base: feature/phase-G-planning
target: develop (via stack A, PR base feature/phase-G-planning)
owner: cfast
# Owner assignment: clint owns most sprints; cfast takes easy closure/fix work; flint takes the harder parallel Stack B track.
---

# Sprint G.0 — Archive Rejected Phase F

## Goal

- cfast executes the archival closure for the rejected Phase F line without
  merging any of its code

## Hard Dependencies

- `integrate/phase-F` and `sprint/F.*` branches (read-only reference)

## Exact Targets

- git tag `archive/phase-F`
- Phase F branches/worktrees and `../sc-lint-worktrees/worktree-tracking.md`

## Deliverables

- Git housekeeping performed by team-lead, recorded in the PR description:
  PR #128 closed unmerged; tag `archive/phase-F` at `integrate/phase-F` head;
  worktrees `sprint/F.*` and `integrate/phase-F` removed; branches deleted
  after tagging; `worktree-tracking.md` updated.

## Unblock Milestone

Create and push the `archive/phase-F` tag at the recorded
`integrate/phase-F` head. Report the tag immediately; G.1 starts from that
tagged archival boundary while G.0 removes the now-archived worktrees and
branches and updates tracking.

## Acceptance Criteria

- `git tag -l archive/phase-F` prints the tag; `git worktree list` shows no
  `phase-F` or `sprint/F.` path.
- No `docs/sc-lint/adr/ADR-014*` file exists on `develop`.

## Required Validation

- `cargo test --workspace` unchanged (docs-only sprint).
- Docs link check: every relative link in `docs/plans/phase-G/*.md` resolves.

## Out Of Scope

- any code recovery (G.1)
- any change under `crates/`, `packages/`, `scripts/`
