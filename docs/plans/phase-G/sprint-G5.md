---
id: G.5
title: Ecosystem Rollout
status: planned
branch: n/a (consumer repositories)
worktree: n/a (target repository worktrees)
stack: external-non-branch
stack_base: n/a
target: each approved consumer's develop (external PRs)
owner: clint
---

# Sprint G.5 — Ecosystem Rollout

## Goal

- adopt the kit in every remaining sc-ecosystem Rust repository using the
  skill alone

## Hard Dependencies

- G.4a, G.4b, and G.4c merged with their required CI and drift checks green.
- D4 is resolved: `docs/sc-lint/adoption.md` contains the approved remaining
  Rust-repository inventory, default branch, and explicit exclusions before
  any rollout PR is opened. This sprint must not infer membership from local
  directories or GitHub search.

## Exact Targets

- one external consumer PR per approved rollout-table row
- `docs/sc-lint/adoption.md` (rollout table updated through a documentation
  PR in this repository; no product or kit code changes)

## Deliverables

- one consumer PR per repository, opened by the adopter agent; PR body
  contains the dry-run exit-0 output.
- a rollout table in `docs/sc-lint/adoption.md`: repository, default branch,
  kit version, PR URL, merge commit, date, dry-run result, CI result, and any
  explicit exclusion/rationale. First-wave rows are entered from the G.4a–G.4c
  retained inputs before remaining-repository rollout begins.

## Acceptance Criteria

- every approved row merges with `just setup`, `just lint`, and `just test`
  green and dry-run exit 0.
- zero product or kit changes in this sprint. Any needed change is a filed
  issue and a G.3-class fix released before the affected repository adopts.

## Out Of Scope

- non-Rust repositories
- discovering or silently expanding the rollout inventory
