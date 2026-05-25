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
- `docs/sc-lint/adr/README.md`
- `docs/sc-lint/version-requirements.md`
- `docs/sc-lint/phase-C-plan.md`
- `docs/sc-lint/crate-architecture.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/project-plan.md`

## Deliverables

- `ADR-011` accepted records:
  - `sc-lint-version` as a planned dedicated workspace crate that owns
    interface-version checks and integrates with the top-level CLI through
    `sc-lint check interfaces`
  - `cargo-semver-checks` as the initial Rust public API decision engine
  - one canonical version-configuration surface under
    `[version.families.<family>]` in `sc-lint` config so configured families
    can be distinguished from omitted families
  - one planned Rust-public-API translation layer in `sc-lint-version` that
    consumes `cargo-semver-checks` machine-readable output and exit-status
    semantics into the multi-family verdict model
  - one explicit ownership split where `sc-lint-version` owns canonical
    interface artifacts and version verdicts, while the shared report layer
    owns reusable HTML/XHTML rendering
- requirements define breaking-change semantics for:
  - Rust public APIs
  - stable top-level CLI commands and machine contracts
  - RPC/socket interfaces
- crate/phase architecture records the committed Phase `C` form-factor and
  invocation shape for version checks:
  - planned dedicated workspace crate: `sc-lint-version`
  - planned top-level invocation command: `sc-lint check interfaces`
- Phase `C` sequence is added to the project planning line with separate
  closures for:
  - policy/baseline definition
  - artifact publication
  - hard-fail gate planning
  - consumer-adoption skill design
  - minimal marketplace publication

## Explicit Code Samples

```bash
sc-lint check interfaces --family rust-public-api --baseline-version 0.2.0
```

```toml
[version.families."rust-public-api"]
enabled = true
baseline = { published = "0.2.0" }

[version.families.cli]
enabled = true
baseline_artifact = "artifacts/baselines/cli-v0.2.0.json"
```

```json
{
  "tool": "cargo-semver-checks",
  "output_mode": "machine-readable-json",
  "adapter": "sc_lint_version::rust_public_api::SemverChecksAdapter",
  "verdict_family": "rust-public-api"
}
```

## This Sprint Does Not Close

- implementation of `sc-lint-version`
- generation of the published HTML/XHTML/JSON interface reports
- CI enforcement of version-check hard failures

## Acceptance Criteria

- `ADR-011` decision text, consequences, and follow-on planning sections stay
  consistent with the `C.1` deliverables for breaking-change semantics, the
  `[version.families.<family>]` config surface, the dedicated `sc-lint-version`
  form-factor, the `sc-lint check interfaces` invocation path, and the
  Rust-public-API translation-layer contract
- `version-requirements.md` defines interface-family-specific breaking-change
  rules
- `version-requirements.md` contains one normative Interface Family
  Identifiers section listing the canonical machine token strings used across
  config, CLI flags, baseline artifacts, and verdict output
- `version-requirements.md` defines the planned `[version.families.<family>]`
  configuration surface, including the distinction between omitted families
  and configured-but-not-present families
- `version-requirements.md`, `phase-C-plan.md`, and
  `crate-architecture.md` all identify `sc-lint-version` as a planned
  dedicated workspace crate invoked through `sc-lint check interfaces`
- the plan defines the `cargo-semver-checks` ingestion contract as a
  `sc-lint-version` translation layer from machine-readable tool output into
  the multi-family verdict model
- `project-plan.md` and `phase-C-plan.md` both show the same `C.1`-`C.5`
  sequence
- the plan explicitly states that generated report packages must follow the
  XHTML fragment/report pattern, must not be hand-written HTML monoliths, and
  must not be implemented as feature-local HTML code inside `sc-lint-version`

## Required Validation

- `just lint`
