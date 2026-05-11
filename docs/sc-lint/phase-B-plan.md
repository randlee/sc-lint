# `sc-lint` Phase B Plan

This document is the execution stub for Phase `B`, the post-Phase-A hardening
and process-tightening phase.

## Objective

Phase `B` turns the recurring systemic findings from Phase `A` into explicit
planned engineering work so the same defect patterns stop reappearing during
later sprint implementation and QA.

## Current Scope

The first planned sprint in this phase is:

- `B.1`
  - post-mortem carry-forwards from Phase `A`
  - systemic lint-gate additions for repeat offender patterns
  - observability boundary-policy ADR work
  - QA-process tightening for rust-best-practices coverage on every sprint
  - see [docs/sc-lint/sprint-B1.md](./sprint-B1.md)

## Phase Structure

Phase `B` starts with one required planning-and-hardening sprint:

1. `B.1`
   - encode Phase-A post-mortem findings as planned product/process work
   - define the next lint gates and architecture-policy follow-ups
   - tighten QA expectations before additional Phase-B feature scope begins

Additional Phase `B` sprint scope may be added after `B.1` is reviewed.

## Exit Direction

Phase `B` should leave the repo with:

- explicit planned ownership for the recurring Phase-A defect families
- a documented ADR track for observability boundary policy beyond the logging
  rollout work
- a standing QA-process expectation that rust-best-practices runs in
  `practice_mode:all` on every sprint

