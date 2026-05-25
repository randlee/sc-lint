---
id: C.3
title: Hard-Fail Version Gate Integration
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.3 — Hard-Fail Version Gate Integration

## Goal

- define how `sc-lint-version` fails on breaking interface changes
- plan local and CI integration points
- ensure all supported interface families contribute to one clear verdict

## Hard Dependencies

- [docs/sc-lint/sprint-C1.md](./sprint-C1.md)
- [docs/sc-lint/sprint-C2.md](./sprint-C2.md)
- [docs/sc-lint/version-requirements.md](./version-requirements.md)

## Exact Targets

- `docs/sc-lint/phase-C-plan.md`
- `docs/sc-lint/sprint-C3.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/project-plan.md`

## Deliverables

- planned hard-fail mode for `sc-lint-version` that reports:
  - interface family
  - baseline source
  - breaking items
  - associated published artifact paths
- planned workflow integration for:
  - local developer execution
  - CI gate usage
  - release review usage
- explicit policy for how no-interface-present families are reported without
  silently dropping them from the published artifact set

## Explicit Code Samples

```json
{
  "ok": false,
  "interface_family": "rust-public-api",
  "baseline": "crates.io:0.2.0",
  "breaking_items": ["function_missing"],
  "artifact_paths": [
    "artifacts/interfaces/crate-api/sc-lint/index.html",
    "artifacts/interfaces/crate-api/sc-lint/index.json"
  ]
}
```

## This Sprint Does Not Close

- implementation of every interface extractor
- release-distribution changes unrelated to version gating
- transport-interface implementation for repos that do not yet define one

## Acceptance Criteria

- the plan defines one hard-fail verdict model across Rust API, CLI, and
  RPC/socket families
- the plan requires concrete artifact-path reporting in failure output
- the plan keeps “not present in this repo” explicit for unsupported current
  interface families instead of omitting them silently

## Required Validation

- `just lint`
