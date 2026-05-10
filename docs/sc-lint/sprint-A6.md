# Sprint A.6 — Boundary Inventory Loader

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.6"
worktree: /Users/randlee/Documents/github/sc-lint
branch: develop
status: planned
estimated_scope: M
```

## Goal

Implement Rust-native TOML boundary inventory loading, schema validation, and
duplicate handling in `sc-lint-boundary`.

## Scope Summary

This sprint is the first half of the Python-to-Rust boundary migration. It
stops before manifest-policy enforcement so the loader, schema, and planning
item semantics can stabilize independently.

## Governing Requirements

- `REQ-PRODUCT-007`
- `REQ-PRODUCT-008`
- `REQ-PRODUCT-009`
- `REQ-PRODUCT-009A`
- `REQ-PRODUCT-014`
- `REQ-PRODUCT-015`
- `REQ-PRODUCT-017`
- `REQ-LOG-004`
- `REQ-LOG-005`

## Governing ADRs

- `docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md`

## Governing Boundaries

- `BOUNDARY-ScLintBoundaryAnalyzer`
- `BOUNDARY-ScLintCli`
- `BOUNDARY-DirectiveModel`

## Prerequisites

- A.1a through A.5 complete
- TOML boundary records remain the canonical source of planning truth
- Python boundary validator remains available

## Hard Dependencies

- do not retire the Python validator in this sprint
- do not conflate loader/schema issues with manifest-policy enforcement

## Non-Goals

- manifest ownership rules
- manifest section rules
- parity retirement

## Sub-Tasks

1. Implement TOML boundary loading
   Development work:
   - load boundary records from `boundaries/`
   - load planning metadata from `boundaries/planning.toml`
   Required tests:
   - fixture tests for valid boundary inventories
   Required doc or boundary updates:
   - keep migration docs aligned with implemented source discovery behavior

2. Implement schema validation
   Development work:
   - validate required fields
   - validate enum/value contracts
   - validate planning-item shape
   Required tests:
   - invalid-schema fixture tests
   Required doc or boundary updates:
   - update docs if field names or item-key semantics narrow

3. Implement duplicate handling
   Development work:
   - detect duplicate boundary ids
   - detect duplicate planned item keys
   - detect invalid owner/path relationships
   Required tests:
   - duplicate-id and duplicate-item-key tests
   Required doc or boundary updates:
   - keep the boundary-enforcement model aligned with actual duplicate
     behavior

4. Plan analyzer logging for `sc-runtime`
   Development work:
   - define `sc-runtime` analyzer entry logging for delegated analyze calls
   - define completion logging with verdict and finding count
   - keep logger lifecycle ownership in the top-level CLI
   Required tests:
   - doc review for backend-service naming and finding-count event consistency
   Required doc or boundary updates:
   - keep `docs/sc-lint/logging.md` aligned with the `sc-runtime` logging
     pattern

## Split Recommendation

Keep A.6 together. Loader, schema validation, and duplicate handling form one
coherent contract surface and should stabilize before manifest-policy work
starts.

## Acceptance Criteria

- `sc-lint-boundary` loads TOML boundary inventories directly
- schema validation is Rust-native
- duplicate boundary and planning-item handling is Rust-native
- the Python validator still exists as a parity oracle

## Required Validation

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `python3 -m unittest discover -s .just/tests -p 'test_*.py'`
- `just lint`

## Required Document Updates

- `docs/sc-lint/boundary-toml-migration.md`
- `docs/sc-lint/boundary-enforcement-model.md`
- `docs/sc-lint/extraction-plan.md`
- `docs/sc-lint/foundation-phase-plan.md`
- `docs/project-plan.md`

## Risks And Watchouts

- do not weaken planning-item traceability during the Rust move
- do not let loader semantics drift from the documented TOML contract
- do not mix manifest-policy failures into loader/schema diagnostics
