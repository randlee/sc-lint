# Sprint A.3 — Generic Utility Extraction

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.3"
worktree: <repo-root>
branch: develop
status: planned
estimated_scope: M
```

## Goal

Extract the first wave of consumer-neutral Python utilities into `sc-lint` and
expose them through the top-level CLI without weakening lint behavior or
consumer configurability.

## Scope Summary

This sprint moves the low-risk generic utilities out of the consumer repo and
into `sc-lint`, while preserving their consumer-neutral semantics and adding
standalone fixture coverage.

## Governing Requirements

- `REQ-PRODUCT-005`
- `REQ-PRODUCT-006`
- `REQ-PRODUCT-013`
- `REQ-CLI-006`
- `REQ-CLI-012`
- `REQ-CLI-013`

## Governing ADRs

- `docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md`

## Governing Boundaries

- `BOUNDARY-ScLintCli`
- `BOUNDARY-ScLintBoundaryAnalyzer`

## Prerequisites

- Sprint A.1b complete with top-level CLI dispatch
- Sprint A.2 complete with stable profile semantics

## Hard Dependencies

- extracted utilities must become consumer-neutral before they are treated as
  product surfaces
- the top-level CLI must remain the coordination layer rather than turning
  Python tools into independent user-facing contracts

## Non-Goals

- forcing simple utilities into Rust
- migrating ATM-specific policy lints unchanged
- migrating boundary inventory logic into Rust in this sprint

## Sub-Tasks

1. Extract line-count lint
   Development work:
   - move the line-count utility into `sc-lint`
   - make thresholds and exemptions config-driven
   - expose it through the top-level CLI
   Required tests:
   - standalone fixture tests
   - CLI dispatch tests
   Required doc or boundary updates:
   - update extraction-plan docs if command names or config keys change

2. Extract identity-literal lint framework
   Development work:
   - move the generic identity/semantic-literal scanner into `sc-lint`
   - keep consumer-specific role-name policy out of the shared default surface
   - document the configurable framework seam
   Required tests:
   - standalone fixture tests
   - config-driven allow/deny tests
   Required doc or boundary updates:
   - update product docs if the framework name differs from the current plan

3. Define Python Adapter Protocol
   Development work:
   - define a standard JSON adapter for Python utilities
   - ensure Python failures are reliably mapped into `CliError` without
     scraping tracebacks
   Required tests:
   - adapter normalization tests
   Required doc or boundary updates:
   - update CLI architecture docs to include the adapter pattern

4. Extract generic view plumbing
   Development work:
   - move the generic report/site plumbing that is not ATM-specific
   - expose it behind the top-level CLI only when the command contract is
     stable enough
   Required tests:
   - fixture tests for output generation
   Required doc or boundary updates:
   - update README and roadmap if view command names narrow

## Split Recommendation

If schedule pressure appears, split A.3 by utility family:

- A.3a line-count + identity literal framework
- A.3b generic view plumbing

The line-count utility should land first because it is the lowest-risk
extraction.

## Acceptance Criteria

- the scheduled generic utilities live in `sc-lint`
- they are covered by standalone fixture tests
- consumer-specific policy does not leak into the shared defaults
- top-level CLI dispatch exists for extracted utilities that are ready for
  stable exposure

## Required Validation

- `python3 -m unittest discover -s .just/tests -p 'test_*.py'`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`

## Required Document Updates

- `docs/sc-lint/extraction-plan.md`
- `docs/sc-lint/foundation-phase-plan.md`
- `docs/project-plan.md`
- `docs/sc-lint/README.md`

## Risks And Watchouts

- do not migrate ATM-specific naming into shared product defaults
- do not expose unstable Python utility output as a public machine contract
- do not let wrapper/orchestration convenience outrun testable fixture coverage
