# Sprint A.5 — Runtime Crate Extraction

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.5"
worktree: <repo-root>
branch: develop
status: planned
estimated_scope: M
```

## Goal

Create `sc-lint-runtime` and import the shared std runtime/concurrency rule
family into its dedicated analyzer crate.

## Scope Summary

This sprint is intentionally limited to the shared std runtime rule family. It
creates `sc-lint-runtime`, imports the proven `SCB-RUNTIME-001/002` rules from
the `atm-core` proving surface, and keeps Tokio-specific work out of scope.

## Governing Requirements

- `REQ-PRODUCT-004B`
- `REQ-PRODUCT-004C`
- `REQ-PRODUCT-006`
- `REQ-PRODUCT-006A`
- `REQ-PRODUCT-006AA`
- `REQ-PRODUCT-015A`
- `REQ-PRODUCT-015B`
- `REQ-PRODUCT-015C`
- `REQ-CLI-007F`
- `REQ-LOG-004`
- `REQ-LOG-005`

## Governing ADRs

- `docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md`

## Governing Boundaries

- `BOUNDARY-ScLintCli`
- `BOUNDARY-ScLintRuntimeAnalyzer`

## Prerequisites

- A.1a through A.4 complete
- `sc-lint-portability` already owns the shared portability family

## Hard Dependencies

- do not place Tokio-specific semantics in `sc-lint-runtime`
- do not change runtime rule ids during the import

## Non-Goals

- Tokio-specific runtime linting
- boundary inventory loader migration
- manifest-policy migration

## Primary Targets

- `Cargo.toml`
- `crates/sc-lint-runtime/`
- `boundaries/sc-lint-runtime/`
- `docs/sc-lint/extraction-plan.md`
- `docs/sc-lint/roadmap.md`
- `docs/sc-lint/README.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Sub-Tasks

1. Create `sc-lint-runtime`
   Development work:
   - add the crate to the workspace
   - define its CLI/library entry points
   Required tests:
   - workspace compile/tests for the new crate
   Required doc or boundary updates:
   - add or update runtime crate references across product docs

2. Import `SCB-RUNTIME-001`
   Development work:
   - port the bare production `Condvar::wait(...)` rule
   Required tests:
   - regression tests from the consumer-proven cases
   Required doc or boundary updates:
   - update rule inventory references if implementation details narrow

3. Import `SCB-RUNTIME-002`
   Development work:
   - port the discarded `wait_timeout*` result rule
   Required tests:
   - regression tests from the consumer-proven cases
   - parity validation against the source implementation in `atm-core`
   Required doc or boundary updates:
   - keep runtime documentation aligned with the proven semantics

4. Reserve Tokio-specific follow-on ownership
   Development work:
   - verify product docs keep Tokio-specific rules out of `sc-lint-runtime`
   - keep release-1 top-level CLI integration delegated unless a later design
     update approves direct `sc-lint` crate linkage
   Required tests:
   - none beyond doc and boundary review
   Required doc or boundary updates:
   - keep `sc-lint-tokio` present as a planned future crate only

5. Plan analyzer logging for `sc-portability`
   Development work:
   - define `sc-portability` analyzer entry logging for delegated analyze
     calls
   - define completion logging with verdict and finding count
   - keep logging initialization out of the backend crate
   Required tests:
   - doc review for backend-service naming and finding-count event consistency
   Required doc or boundary updates:
   - keep `docs/sc-lint/logging.md` aligned with the `sc-portability`
     logging pattern

## Split Recommendation

Keep A.5 together. The std runtime rule family is small enough to migrate in
one contained sprint and should not be split across two crates or two phases.

## Acceptance Criteria

- `sc-lint-runtime` exists in the workspace
- `SCB-RUNTIME-001` and `SCB-RUNTIME-002` live in `sc-lint-runtime`
- product docs clearly separate generic std runtime rules from future
  Tokio-specific rules

## Required Validation

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`

## Required Document Updates

- `docs/sc-lint/extraction-plan.md`
- `docs/sc-lint/foundation-phase-plan.md`
- `docs/sc-lint/roadmap.md`
- `docs/sc-lint/README.md`
- `docs/project-plan.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Risks And Watchouts

- do not let runtime become a disguised Tokio crate
- do not mix boundary-inventory work into this sprint
- do not let subset lint names replace the primary runtime crate identity
