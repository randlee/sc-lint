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
  "families": [
    {
      "interface_family": "rust-public-api",
      "baseline": "crates.io:0.2.0",
      "breaking_items": ["function_missing"],
      "artifact_paths": [
        "artifacts/interfaces/crate-api/sc-lint/index.html",
        "artifacts/interfaces/crate-api/sc-lint/index.json"
      ]
    },
    {
      "interface_family": "cli",
      "baseline": "artifacts/baselines/cli-v0.2.0.json",
      "breaking_items": [],
      "artifact_paths": [
        "artifacts/interfaces/cli/index.html",
        "artifacts/interfaces/cli/index.json"
      ]
    },
    {
      "interface_family": "rpc-socket",
      "status": "not_present",
      "breaking_items": [],
      "artifact_paths": []
    }
  ]
}
```

## This Sprint Does Not Close

- implementation of every interface extractor
- release-distribution changes unrelated to version gating
- transport-interface implementation for repos that do not yet define one

## Acceptance Criteria

- `docs/sc-lint/version-requirements.md` defines one hard-fail verdict model
  across Rust API, CLI, and RPC/socket families
- the sprint code sample and `docs/sc-lint/version-requirements.md` both show
  the aggregate top-level `ok` rollup plus per-family verdict entries for a
  multi-family run
- the sprint code sample and `docs/sc-lint/version-requirements.md` both
  require concrete published-artifact paths in failure output for present
  families
- `docs/sc-lint/version-requirements.md` keeps “not present in this repo”
  explicit for unsupported current interface families instead of omitting them
  silently

## Required Validation

- `just lint`
