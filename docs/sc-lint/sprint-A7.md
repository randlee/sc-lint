# Sprint A.7 — Manifest Policy And Parity

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.7"
worktree: <repo-root>
branch: feature/sprint-A7
status: in-progress
estimated_scope: M
```

## Goal

Implement Rust-native manifest-policy enforcement in `sc-lint-boundary` and
keep the Python manifest-policy validator as the parity oracle until behavior
is proven stable.

## Scope Summary

This sprint completes the release-1 boundary migration path. It assumes the
Rust loader/schema work from A.6 already exists and focuses on manifest
ownership rules, manifest-section rules, and parity comparison.

The current parity window is intentionally narrow:

- Rust now owns the workspace-package inheritance checks from
  `.just/lint_manifests.py`
- Rust now owns the internal workspace path-dependency version-pin checks from
  `.just/lint_manifests.py`
- broader boundary-inventory parity remains an A.6 foundation concern plus a
  later inventory-parity enforcement stage

## Governing Requirements

- `REQ-PRODUCT-014`
- `REQ-PRODUCT-015`
- `REQ-PRODUCT-017`
- `REQ-LOG-004`
- `REQ-LOG-005`

## Governing ADRs

- `docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md`

## Governing Boundaries

- `BOUNDARY-ScLintBoundaryAnalyzer`

## Prerequisites

- A.6 complete with Rust-native boundary loading/schema validation
- Python manifest-policy validator still available as the reference path

## Hard Dependencies

- do not retire the Python validator until parity is proven stable
- do not change canonical TOML semantics while parity comparison is underway

## Non-Goals

- retirement of the Python validator
- new analyzer-family imports
- graph-db integration

## Primary Targets

- `crates/sc-lint-boundary/`
- `.just/tests/`
- `docs/sc-lint/extraction-plan.md`
- `docs/sc-lint/boundary-toml-migration.md`
- `docs/sc-lint/boundary-enforcement-model.md`
- `docs/sc-lint/foundation-phase-plan.md`
- `docs/sc-lint/roadmap.md`
- `docs/project-plan.md`

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
   - port the current package-section workspace-inheritance rules into Rust
   Required tests:
   - fixture workspace pass/fail tests
   Required doc or boundary updates:
   - keep section-policy docs aligned with the implemented inheritance checks

3. Add parity validation
   Development work:
   - compare Rust and Python results across representative fixtures
   - document any intentional divergence explicitly
   Required tests:
   - parity harness tests
   Required doc or boundary updates:
   - keep extraction/migration docs aligned with the parity window policy

4. Plan manifest-policy logging
   Development work:
   - log one entry event for manifest-policy tool execution with the effective
     config/parity mode used by the CLI
   - log one completion event with verdict and elapsed time in ms
   - log one error event per `CliError` after top-level normalization, while
     keeping logger initialization in the CLI
   Required tests:
   - doc review proving manifest-policy execution follows the standard
     entry/exit/error pattern
   Required doc or boundary updates:
   - keep `docs/sc-lint/logging.md` aligned with the manifest-policy tool path

## Split Recommendation

Keep A.7 together. Manifest-policy migration without the parity window would
leave the product in an ambiguous transition state.

## Acceptance Criteria

- manifest ownership and section rules exist in Rust
- parity comparison to the Python manifest-policy validator exists
- Python remains the oracle until parity is approved stable
- docs explicitly state what remains Python-backed and what has become
  Rust-native
- manifest-policy entry/exit/error events follow the standard CLI event
  pattern and do not initialize the logger in backend code

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

## Sub-Task 4 Review Artifact

Manifest-policy logging review for A.7:

- `crates/sc-lint/src/main.rs` remains the only logger initialization point
- `crates/sc-lint/src/logging.rs` emits the standard CLI
  `cli.command.started`, `cli.command.completed`, and `cli.command.error`
  events for `lint.sc-boundary`
- those `sc-boundary` events now carry the manifest-policy migration metadata
  fields `manifest_policy_mode = "rust-native"` and
  `manifest_policy_parity = "python-oracle"`
- `docs/sc-lint/logging.md` is aligned with the same command path and field
  names
