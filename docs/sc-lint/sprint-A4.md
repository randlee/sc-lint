# Sprint A.4 — Portability Crate Extraction

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.4"
worktree: <repo-root>
branch: develop
status: planned
estimated_scope: M
```

## Goal

Create `sc-lint-portability` and move the shared portability rule family into
its dedicated analyzer crate.

## Scope Summary

This sprint is intentionally crate-scoped. It creates the portability tool
crate, moves the existing portability implementation out of
`sc-lint-boundary`, and imports the additional proven portability rules from
`atm-core`.

## Governing Requirements

- `REQ-PRODUCT-004A`
- `REQ-PRODUCT-006`
- `REQ-PRODUCT-006A`
- `REQ-PRODUCT-006AA`
- `REQ-PRODUCT-006B`
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
- `BOUNDARY-ScLintBoundaryAnalyzer`
- `BOUNDARY-ScLintPortabilityAnalyzer`

## Prerequisites

- A.1a through A.3 complete
- current `PORT-001/002/003` behavior remains green in the repo gate

## Hard Dependencies

- do not grow `sc-lint-boundary` with new portability rules once this sprint
  starts
- do not change rule ids during the crate move

## Non-Goals

- runtime/concurrency rule migration
- boundary inventory loader migration
- manifest-policy migration
- Tokio-specific rule work

## Primary Targets

- `Cargo.toml`
- `crates/sc-lint-portability/`
- `crates/sc-lint-boundary/`
- `.just/lint_sc_portability.py`
- `.just/run_lint.py`
- `boundaries/sc-lint-portability/`
- `docs/sc-lint/extraction-plan.md`
- `docs/sc-lint/roadmap.md`
- `docs/sc-lint/README.md`
- `docs/requirements.md`
- `docs/architecture.md`

## Sub-Tasks

1. Create `sc-lint-portability`
   Development work:
   - add the new crate to the workspace
   - define its CLI/library entry points
   - define its current direct dependencies
   Required tests:
   - workspace compile/tests for the new crate
   Required doc or boundary updates:
   - add or update portability crate references in requirements, architecture,
     and roadmap docs

2. Move existing portability rules
   Development work:
   - move `PORT-001`
   - move `PORT-002`
   - move `PORT-003`
   from the current boundary analyzer path into `sc-lint-portability`
   Required tests:
   - rule regression tests for all moved rules
   Required doc or boundary updates:
   - update docs that still describe these rules as temporary

3. Import proven portability rules
   Development work:
   - port `PORT-004`
   - port `PORT-005`
   into `sc-lint-portability`
   Required tests:
   - consumer-proven fixture or regression coverage for both imported rules
   - parity validation against the source implementation in `atm-core`
   Required doc or boundary updates:
   - update rule inventory references

4. Retarget wrapper and CLI ownership references
   Development work:
   - retarget `sc-portability` wrappers to the new crate
   - keep `sc-boundary` focused on boundary ownership only
   - keep release-1 top-level CLI integration delegated unless a later design
     update approves direct `sc-lint` crate linkage
   Required tests:
   - local lint target coverage for `sc-portability`
   Required doc or boundary updates:
   - keep the top-level CLI docs aligned so primary lint targets map to the
     backend crate boundary

5. Plan analyzer logging baseline for `sc-portability`
   Development work:
   - keep the logging ownership boundary at the `analyze_workspace` seam:
     the top-level CLI initializes the logger and analyzer crates only emit
     structured events through log macros inside the delegated analysis path
   - define `sc-portability` analyzer entry logging for delegated analyze
     calls
   - define completion logging with verdict and finding count
   - keep emission ownership in the top-level CLI logging layer and log only
     after result normalization through `CommandEnvelope<T>` or `CliError`
   Required tests:
   - doc review for backend-service naming and finding-count event consistency
   Required doc or boundary updates:
   - keep `docs/sc-lint/logging.md` aligned with the `sc-portability`
     pattern

## Split Recommendation

Keep A.4 together. The portability crate should land with all five shared
portability rules rather than leaving the rule family split across crates.

## Acceptance Criteria

- `sc-lint-portability` exists in the workspace
- `PORT-001` through `PORT-005` live in `sc-lint-portability`
- `sc-lint-boundary` no longer owns portability-rule business logic
- wrapper and CLI docs identify portability as its own backend surface
- `sc-lint-portability` does not initialize the logger runtime and relies on
  CLI-owned logging hooks only
- portability-tool entry/exit/error events are emitted only after top-level
  normalization through `CommandEnvelope<T>` or `CliError`

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

- do not leave the same portability rule family partially owned by two crates
- do not invent Tokio-specific abstractions in the generic portability crate
- do not let convenience wrapper names replace clear crate ownership in the
  primary command surface
