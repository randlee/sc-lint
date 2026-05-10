# Sprint A.7 — Manifest Policy And Parity

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.7"
worktree: <repo-root>
branch: develop
status: planned
estimated_scope: M
```

## Goal

Implement Rust-native manifest-policy enforcement in `sc-lint-boundary` and
keep the Python boundary validator as the parity oracle until behavior is
proven stable.

## Scope Summary

This sprint completes the release-1 boundary migration path. It assumes the
Rust loader/schema work from A.6 already exists and focuses on manifest
ownership rules, manifest-section rules, and parity comparison.

## Governing Requirements

- `REQ-PRODUCT-014`
- `REQ-PRODUCT-015`
- `REQ-PRODUCT-017`

## Governing ADRs

- `docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md`

## Governing Boundaries

- `BOUNDARY-ScLintBoundaryAnalyzer`

## Prerequisites

- A.6 complete with Rust-native boundary loading/schema validation
- Python boundary validator still available as the reference path

## Hard Dependencies

- do not retire the Python validator until parity is proven stable
- do not change canonical TOML semantics while parity comparison is underway

## Non-Goals

- retirement of the Python validator
- new analyzer-family imports
- graph-db integration

## Sub-Tasks

1. Implement manifest ownership rules
   Development work:
   - port dependency ownership rules into Rust
   Required tests:
   - fixture workspace pass/fail tests
   Required doc or boundary updates:
   - update rule documentation if ownership semantics narrow

2. Implement manifest section rules
   Development work:
   - port manifest-section placement rules into Rust
   Required tests:
   - fixture workspace pass/fail tests
   Required doc or boundary updates:
   - keep section-policy docs aligned with implemented checks

3. Add parity validation
   Development work:
   - compare Rust and Python results across representative fixtures
   - document any intentional divergence explicitly
   Required tests:
   - parity harness tests
   Required doc or boundary updates:
   - keep extraction/migration docs aligned with the parity window policy

## Split Recommendation

Keep A.7 together. Manifest-policy migration without the parity window would
leave the product in an ambiguous transition state.

## Acceptance Criteria

- manifest ownership and section rules exist in Rust
- parity comparison to the Python validator exists
- Python remains the oracle until parity is approved stable
- docs explicitly state what remains Python-backed and what has become
  Rust-native

## Required Validation

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `python3 -m unittest discover -s .just/tests -p 'test_*.py'`
- `just lint`

## Required Document Updates

- `docs/sc-lint/extraction-plan.md`
- `docs/sc-lint/boundary-toml-migration.md`
- `docs/sc-lint/boundary-enforcement-model.md`
- `docs/sc-lint/foundation-phase-plan.md`
- `docs/project-plan.md`
- `docs/sc-lint/roadmap.md`

## Risks And Watchouts

- do not declare parity complete without explicit comparison evidence
- do not let consumer-repo edge cases vanish during fixture reduction
- do not treat Python parity as a permanent architecture instead of a
  migration window
