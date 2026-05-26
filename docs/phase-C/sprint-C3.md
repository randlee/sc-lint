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

- [docs/phase-C/sprint-C1.md](./sprint-C1.md)
- [docs/phase-C/sprint-C2.md](./sprint-C2.md)
- [docs/sc-lint-version/requirements.md](../sc-lint-version/requirements.md)

## Exact Targets

- `docs/phase-C/phase-C-plan.md`
- `docs/phase-C/sprint-C3.md`
- `docs/sc-lint-version/requirements.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/project-plan.md`

## Deliverables

- planned hard-fail mode for `sc-lint-version` that reports:
  - interface family
  - baseline source
  - breaking items
  - associated published artifact paths
- planned family-selection contract for hard-fail runs:
  - `sc-lint-version` reads configured families from
    `[version.families.<family>]`
  - omitted family tables are outside the run
  - configured families with no current repo surface emit `not_present`
- planned workflow integration for:
  - local developer execution
  - CI gate usage
  - release review usage
- planned Rust-public-API ingestion contract for the hard-fail path:
  - `sc-lint-version` consumes `cargo-semver-checks` machine-readable output
    and exit-status semantics
  - one adapter translates tool findings into the `REQ-VERSION-017A`
    `breaking_items` record for the `rust-public-api` family
- planned CLI-baseline usage contract for the hard-fail path:
  - the `cli` family compares current extracted contract artifacts against the
    versioned baseline artifact defined in `C.2`
  - failure output names the baseline artifact path that was evaluated
- concise top-level requirements and architecture updates that keep the
  hard-fail gate visible outside the Phase `C` sprint docs:
  - `docs/requirements.md` states that interface-version checks can fail local
    and CI workflows across all supported interface families
  - `docs/architecture.md` states that the planned versioning layer owns one
    aggregate multi-family hard-fail verdict for canonical interface artifacts
    and their published reports
- explicit policy for how no-interface-present families are reported without
  silently dropping them from the published artifact set

## Explicit Code Samples

```json
{
  "ok": true,
  // outer ok = command execution succeeded without CliError
  "command": "check.interfaces",
  "data": {
    // data.ok = business verdict; false means a breaking change was detected
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
        "baseline": {
          "schema": "sc-lint-cli-interface-v1",
          "artifact": "artifacts/baselines/cli-v0.2.0.json"
        },
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
  },
  "diagnostics": []
}
```

## This Sprint Does Not Close

- implementation of every interface extractor
- release-distribution changes unrelated to version gating
- transport-interface implementation for repos that do not yet define one

## Acceptance Criteria

- `docs/sc-lint-version/requirements.md` is updated to define one hard-fail
  verdict model across Rust API, CLI, and RPC/socket families
- `docs/sc-lint-version/requirements.md` is updated to define the
  `[version.families.<family>]` configuration surface and use it to
  distinguish omitted families from configured-but-not-present families
- `docs/sc-lint-version/requirements.md` is updated to define the aggregate
  top-level `ok` rollup plus per-family verdict entries for a multi-family
  run, and the sprint code sample is updated to illustrate a conforming
  example
- `docs/sc-lint-version/requirements.md` is updated to define the
  `cargo-semver-checks` ingestion contract for the `rust-public-api` family
  rather than leaving the `breaking_items` translation implicit
- `docs/sc-lint-version/requirements.md` is updated to reference the versioned
  CLI baseline artifact schema and the `C.2` baseline-generation workflow for
  the `cli` family
- `docs/sc-lint-version/requirements.md` is updated to state explicitly that
  the multi-family verdict is carried under the existing top-level
  `CommandEnvelope<T>` / `CliError` CLI contract rather than a parallel
  machine-readable envelope
- `docs/sc-lint-version/requirements.md` is updated to require concrete
  published-artifact paths in failure output for present families, and the
  sprint code sample is updated to illustrate a conforming example
- `docs/sc-lint-version/requirements.md` is updated to keep “not present in
  this repo” explicit for unsupported current interface families instead of
  omitting them silently
- `docs/requirements.md` and `docs/architecture.md` are updated to carry one
  concise Phase `C`-level hard-fail gate statement consistent with
  `docs/sc-lint-version/requirements.md`

## Required Validation

- `just lint`
