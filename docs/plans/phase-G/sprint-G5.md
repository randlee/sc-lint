---
id: G.5
title: Ecosystem Rollout
status: planned
branch: n/a (consumer repositories)
target: each consumer's develop
---

# Sprint G.5 — Ecosystem Rollout

## Goal

- adopt the kit in every remaining sc-ecosystem Rust repository using the
  skill alone

## Hard Dependencies

- G.4 merged in all three first-wave repositories
- repository list supplied by the user at sprint start

## Deliverables

- one consumer PR per repository, opened by the adopter agent; PR body
  contains the dry-run exit-0 output.
- a rollout table in `docs/sc-lint/adoption.md`: repository, kit version,
  PR, date.

## Acceptance Criteria

- every listed repository merges with `just lint`/`just test` green and
  dry-run exit 0.
- zero product or kit changes in this sprint. Any needed change is a filed
  issue and a G.3-class fix released before the affected repository adopts.

## Out Of Scope

- non-Rust repositories
