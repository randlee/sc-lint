# Sprint S.4 — Rust Boundary Inventory Migration

```yaml
plan_type: sprint_plan
phase: S
sprint: "S.4"
worktree: /Users/randlee/Documents/github/sc-lint
branch: develop
status: planned
estimated_scope: L
```

## Goal

Move boundary inventory and manifest-policy enforcement into
`sc-lint-boundary`, while preserving Python parity validation and folding in
the reusable postmortem analyzer families that are already proven in
`atm-core`.

## Scope Summary

This sprint is the largest release-1 foundation sprint. It moves the canonical
boundary loading and manifest-policy logic into Rust, keeps the Python path as
an oracle during the migration window, and brings the consumer-proven reusable
analyzer families into the shared boundary analyzer.

## Governing Requirements

- `REQ-PRODUCT-004`
- `REQ-PRODUCT-006A`
- `REQ-PRODUCT-007`
- `REQ-PRODUCT-008`
- `REQ-PRODUCT-009`
- `REQ-PRODUCT-009A`
- `REQ-PRODUCT-014`
- `REQ-PRODUCT-015`
- `REQ-PRODUCT-015A`

## Governing ADRs

- `docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md`
- `docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md`

## Governing Boundaries

- `BOUNDARY-ScLintBoundaryAnalyzer`
- `BOUNDARY-ScLintCli`
- `BOUNDARY-DirectiveModel`

## Prerequisites

- S.1 through S.3 complete
- TOML boundary records remain the canonical source of planning truth
- Python boundary validator still available as a parity oracle

## Hard Dependencies

- do not retire the Python boundary path before parity validation is stable
- do not move consumer-local policy lints into `sc-lint` unchanged

## Non-Goals

- full graph-database integration
- replacing every Python utility
- migrating ATM-local fixed-sleep, duplicate semantic string, or TTL triage
  policy directly into `sc-lint`

## Sub-Tasks

1. Implement TOML boundary inventory loading in Rust
   Development work:
   - load boundary records and planning metadata in `sc-lint-boundary`
   - validate schema and duplicate-source/id handling
   Required tests:
   - fixture tests for valid and invalid TOML inventories
   - duplicate-id and duplicate-item-key tests
   Required doc or boundary updates:
   - keep migration docs aligned with the implemented loader behavior

2. Implement manifest ownership and manifest-section rules in Rust
   Development work:
   - port dependency ownership rules
   - port manifest-section placement rules
   - emit stable Rust-native findings
   Required tests:
   - fixture workspaces with manifest-policy pass/fail cases
   Required doc or boundary updates:
   - update rule documentation if rule ids or scopes narrow

3. Keep Python parity validation during migration
   Development work:
   - preserve the Python validator as a reference path
   - add parity harnesses comparing Rust and Python outputs
   Required tests:
   - parity tests on representative fixture inventories
   Required doc or boundary updates:
   - document any intentional divergence explicitly

4. Backport reusable consumer-proven analyzer families
   Development work:
   - migrate:
     - `PORT-004`
     - `PORT-005`
     - `SCB-RUNTIME-001`
     - `SCB-RUNTIME-002`
   - keep rule ids stable
   Required tests:
   - analyzer tests for each migrated rule family
   - regression tests copied from consumer-proven cases
   Required doc or boundary updates:
   - update requirements, roadmap, and rule docs with the migrated families

## Split Recommendation

S.4 may split if needed:

- S.4A boundary inventory + manifest-policy migration
- S.4B reusable analyzer-family backports

If split, S.4A must land first so the Rust boundary loader exists before more
rules rely on it.

## Acceptance Criteria

- `sc-lint-boundary` loads TOML boundary inventories directly
- manifest-policy enforcement exists in Rust
- Python parity validation still exists during the migration window
- reusable `PORT-004/005` and `SCB-RUNTIME-001/002` live in `sc-lint-boundary`
- docs explicitly state what remains Python-only and what is now Rust-native

## Required Validation

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `python3 -m unittest discover -s .just/tests -p 'test_*.py'`
- `just lint`

## Required Document Updates

- `docs/sc-lint/extraction-plan.md`
- `docs/sc-lint/foundation-phase-plan.md`
- `docs/sc-lint/boundary-toml-migration.md`
- `docs/sc-lint/boundary-enforcement-model.md`
- `docs/sc-lint/roadmap.md`
- `docs/project-plan.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Risks And Watchouts

- do not weaken boundary semantics during the Python-to-Rust transition
- do not lose item-level planning/enforcement granularity
- do not backport consumer-local governance rules as if they were shared
  analyzer logic
