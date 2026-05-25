---
id: C.1
title: sc-lint-version Policy And Baseline Definition
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.1 — sc-lint-version Policy And Baseline Definition

## Goal

- define the initial `sc-lint-version` approach
- lock the Rust public API comparison engine and baseline strategy
- define what counts as a breaking change per supported interface family

## Hard Dependencies

- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/project-plan.md](../project-plan.md)
- [docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md](./adr/ADR-011-interface-versioning-and-published-artifacts.md)
- [docs/sc-lint/version-requirements.md](./version-requirements.md)

## Exact Targets

- `docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md`
- `docs/sc-lint/version-requirements.md`
- `docs/sc-lint/phase-C-plan.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/project-plan.md`

## Deliverables

- `ADR-011` draft records:
  - `sc-lint-version` as the planned owner of interface-version checks
  - `cargo-semver-checks` as the initial Rust public API decision engine
  - generated HTML/XHTML/JSON published artifacts as the canonical reporting
    model
- requirements define breaking-change semantics for:
  - Rust public APIs
  - stable top-level CLI commands and machine contracts
  - RPC/socket interfaces
- Phase `C` sequence is added to the project planning line

## Explicit Code Samples

```bash
cargo semver-checks --baseline-version 0.2.0
```

```json
{
  "interface_family": "cli",
  "breaking_change": "required_response_field_removed",
  "command": "lint.sc-boundary"
}
```

## This Sprint Does Not Close

- implementation of `sc-lint-version`
- generation of the published HTML/XHTML/JSON interface reports
- CI enforcement of version-check hard failures

## Acceptance Criteria

- `ADR-011` exists and stays in Draft status
- `version-requirements.md` defines interface-family-specific breaking-change
  rules
- `project-plan.md` and `phase-C-plan.md` both show the same `C.1`-`C.3`
  sequence
- the plan explicitly states that generated report packages must follow the
  XHTML fragment/report pattern and must not be hand-written HTML monoliths

## Required Validation

- `just lint`
