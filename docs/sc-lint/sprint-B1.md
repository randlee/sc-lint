---
id: B.1
title: Carry-Forward Lint-Gate And Portability Scope Hardening
status: planned
branch: feature/phase-B-sprint-plans
worktree: <repo-worktree>/feature/phase-B-sprint-plans
target: develop
---

# Sprint B.1 — Carry-Forward Lint-Gate And Portability Scope Hardening

## Goal

- turn recurring Phase `A` findings into one explicit planned lint-gate
  backlog rather than ad hoc future cleanup
- harden the shared portability scope so cross-platform path, env, and shell
  drift are planned product work rather than review-only findings
- close the scope-definition loop before later Phase `B` implementation sprints
  begin

## Hard Dependencies

- [docs/sc-lint/phase-B-plan.md](./phase-B-plan.md)
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/sc-lint/adr/ADR-007-analyzer-crate-partition.md](./adr/ADR-007-analyzer-crate-partition.md)
- [crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)

## Exact Targets

- `docs/sc-lint/phase-B-plan.md`
- `docs/sc-lint/sprint-B1.md`
- `docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md`
- `docs/sc-lint/adr/README.md`
- `docs/project-plan.md`
- `docs/sc-lint/roadmap.md`
- `docs/sc-lint/README.md`
- `docs/sc-lint/crate-architecture.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- the recurring shared lint-gate backlog is explicitly planned as the product
  response to reusable consumer-proven gaps, without importing
  consumer-specific wrapper names or repo-local report formats, for:
  - raw identity string literals without named constants
  - `/tmp/` paths without intent comments
  - public API error types exposing `anyhow::Error`
  - duplicated `CrateId` newtypes across workspace crates
  - `clippy::for_kv_map` and similar structural for-loop anti-patterns
  - `pub` visibility exceeding the documented contract surface
  - raw `String` fields used for structured identifiers such as `boundary_id`,
    sprint ids, owner ids, and planning keys
- the shared portability backlog is explicitly planned in
  `sc-lint-portability` for:
  - Windows-only path literal parity with the current Unix-only path checks
  - broader cross-platform environment-variable portability rules
  - shell-portability checks for OS-specific shell and command assumptions
- `ADR-010` records the portability-scope and parity decision so the new
  cross-platform rule families stay in `sc-lint-portability` and
  consumer-specific wrappers stay out of the core product

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
let tmp = std::path::PathBuf::from(r"C:\Temp\example.txt");
// Planned shared portability target: Windows-only path literal parity
// companion to the existing Unix-only absolute-path checks.
```

```rust
let shell = "/bin/bash";
// Planned shared portability target: shell-portability lint coverage for
// OS-specific shell-path assumptions in portable code paths.
```

```rust
let home = std::env::var("USERPROFILE");
// Planned shared portability target: broader environment-variable portability
// checks beyond the current home-dir override and set_var-specific coverage.
```

## This Sprint Does Not Close

- implementation of any new lint family named here
- observability boundary-policy ADR acceptance
- QA-process prompt or workflow hardening
- Homebrew release/distribution work

## Acceptance Criteria

- the sprint names the seven recurring shared lint-gate families and the three
  agreed cross-platform portability follow-ons without claiming they are
  already implemented
- `docs/requirements.md` and `docs/architecture.md` both align on
  `sc-lint-portability` as the shared owner of future Windows-path, env, and
  shell portability rules
- `docs/sc-lint/crate-architecture.md` and `docs/sc-lint/README.md` both align
  with the same shared portability ownership and do not describe repo-local
  wrapper surfaces as core product deliverables
- `ADR-010` exists and records:
  - shared portability ownership in `sc-lint-portability`
  - parity expectations between Unix-only and Windows-only path-literal checks
  - the rule that consumer-specific portability wrappers do not migrate
    unchanged into the core product
- no Phase `B` planning doc still claims that observability ADR work or QA
  process hardening closes inside `B.1`

## Required Validation

- `just lint`
