# `sc-lint` Phase D Plan

This document is the planning stub for Phase `D`, the boundary-inventory
improvement phase.

## Objective

Phase `D` starts by turning package-level dependency policy in boundary
inventory from parsed-but-inert metadata into shipped analyzer behavior. The
first sprint closes the current direct-workspace package-edge enforcement gap in
`sc-lint-boundary`, keeps that policy separate from manifest-hygiene checks,
and locks the first production-ready dependency-rule contract for
`allowed_dependencies`, `allowed_dependents`, and `forbidden_edges`.

## Planned Scope

The planned sprints in this phase are:

- `D.1`
  - direct workspace package-edge enforcement in `sc-lint-boundary`
  - stable dependency rule family for:
    - direct dependency not in owner allowlist
    - direct dependent not in owner allowlist
    - explicit forbidden package edge present
  - direct-edge scope includes `[dependencies]`, `[dev-dependencies]`,
    `[build-dependencies]`, and target-specific dependency sections when they
    reference another current workspace member
  - validated dependency-policy parsing at the inventory boundary
  - dedicated package-policy analysis path separate from manifest-hygiene
    checks
  - dedicated operator-visible `dependencies` rule filter separate from both
    source-graph boundary checks and manifest policy
  - stable text/JSON/top-level CLI surfacing through `sc-lint lint sc-boundary`
  - see [docs/phase-D/sprint-D1.md](./sprint-D1.md)

Additional Phase `D` sprint scope remains intentionally open until `D.1`
hardens and any remaining boundary-inventory follow-on work is narrow enough to
split cleanly.

## Phase Structure

1. `D.1`
   - define and implement direct workspace package-edge enforcement from Cargo
     metadata plus TOML boundary inventory
   - include direct workspace edges declared under normal, dev, build, and
     target-specific dependency sections
   - keep direct package policy separate from source-reference rules and
     separate from manifest-workspace hygiene rules, including a dedicated
     operator-visible dependency-rule filter
   - close the current package-policy gap before planning broader
     reachability/parity work

## Exit Direction

Phase `D` should leave the repo with:

- a production-ready plan for enforcing package-level architectural dependency
  policy from boundary inventory
- one explicit dependency-rule family in `sc-lint-boundary` for direct
  workspace package edges
- a locked execution split where:
  - source graph rules remain in the existing boundary-analysis path
  - manifest workspace/version checks remain in manifest policy
  - package dependency policy is enforced in its own dedicated analysis path
    and its own dedicated operator-visible dependency-rule filter
- a clear basis for later Phase `D` follow-on planning only if direct-edge
  enforcement reveals narrower remaining inventory gaps such as transitive
  reachability or broader planning-aware parity work
