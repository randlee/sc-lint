# Sprint A.1b — Top-Level CLI Config And First Backend Dispatch

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.1b"
worktree: <repo-root>
branch: develop
status: planned
estimated_scope: S
```

## Goal

Turn the A.1a contract into a working end-to-end CLI path by adding top-level
config loading and one real delegated backend integration.

## Scope Summary

This sprint completes the first operational `sc-lint` command path. It keeps
config loading at the top level, normalizes one backend through the canonical
envelope, and leaves the repo with a real delegated CLI flow before profile and
utility extraction work begins.

## Governing Requirements

- `REQ-PRODUCT-001`
- `REQ-PRODUCT-002`
- `REQ-PRODUCT-002A`
- `REQ-PRODUCT-002B`
- `REQ-PRODUCT-002C`
- `REQ-PRODUCT-002D`
- `REQ-PRODUCT-002DA`
- `REQ-PRODUCT-002E`
- `REQ-CLI-001`
- `REQ-CLI-002`
- `REQ-CLI-003`
- `REQ-CLI-004`
- `REQ-CLI-005A`
- `REQ-CLI-005B`
- `REQ-CLI-005C`
- `REQ-CLI-005E`
- `REQ-CLI-005F`
- `REQ-CLI-005G`
- `REQ-CLI-007`
- `REQ-CLI-007B`
- `REQ-CLI-007C`
- `REQ-CLI-007D`
- `REQ-CLI-007E`
- `REQ-CLI-008A`
- `REQ-CLI-008D`
- `REQ-CLI-008F`
- `REQ-LOG-001`
- `REQ-LOG-002`
- `REQ-LOG-003`
- `REQ-LOG-004`
- `REQ-LOG-005`

## Governing ADRs

- `docs/sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md`
- `docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md`
- `docs/sc-lint/adr/ADR-008-sc-observability-logging.md`

## Governing Boundaries

- `BOUNDARY-ScLintCli`
- `BOUNDARY-DirectiveModel`
- `BOUNDARY-ScLintBoundaryAnalyzer`

## Prerequisites

- Sprint A.1a complete with a real `sc-lint` crate
- `CommandEnvelope<T>` and `CliError` agreed and testable
- the A.1a contract-review checkpoint completed against Workstreams 4-7

## Hard Dependencies

- config must be loaded once at the top level rather than re-parsed ad hoc in
  backends
- the first delegated backend path must normalize through the A.1a contract
- backend protocol and execution failures must remain explicit top-level error
  cases

## Non-Goals

- implementing every delegated backend command
- adding `xwin` support
- migrating Python boundary logic into Rust
- changing backend crate ownership or introducing backend cross-dependencies

## Primary Targets

- `crates/sc-lint/`
- `crates/sc-lint-boundary/`
- `docs/sc-lint/cli-requirements.md`
- `docs/sc-lint/cli-architecture.md`
- `docs/sc-lint/cli-contract.md`
- `docs/architecture.md`
- `docs/project-plan.md`

## Sub-Tasks

1. Implement top-level config loading
   Development work:
   - define the repo-root discovery path
   - implement one top-level config loader for CLI-owned commands
   - keep command-family config resolution in shared helpers rather than
     per-command parsing branches
   - keep backend-specific config parsing behind the CLI contract seam
   Required tests:
   - repo-root discovery tests
   - malformed-config tests
   Required doc or boundary updates:
   - update CLI architecture docs if the config entry point name changes

2. Implement first backend dispatch seam
   Development work:
   - add one real delegated command path through the CLI, preferably
     `sc-lint lint sc-boundary`
   - normalize backend machine output into the top-level envelope
   - route delegated results through one shared normalization helper used by
     every non-interactive command family
   - handle backend protocol and execution failures explicitly
   Required tests:
   - delegated success-path tests
   - malformed backend JSON tests
   - backend execution failure tests
   - contract-parity tests proving delegated command paths still use the same
     `command`, success-envelope, and `CliError` pattern as direct CLI-owned
     commands
   Required doc or boundary updates:
   - update CLI contract docs if the normalization rules need narrower wording

3. Plan dispatch-seam logging
   Development work:
   - log delegated backend dispatch start for the active command
   - log the normalized delegated result summary after completion
   - keep backend logging as a CLI-owned concern rather than a backend-owned
     logger initialization path
   Required tests:
   - doc review for dispatch/event-shape consistency
   Required doc or boundary updates:
   - keep `docs/sc-lint/logging.md` aligned with delegated dispatch behavior

## Split Recommendation

Keep A.1b together. Config loading and the first delegated backend path should
land in the same sprint so the product exits the bootstrap phase with a real
end-to-end CLI seam.

## Acceptance Criteria

- repo-root discovery and CLI-owned config loading exist
- at least one real delegated backend command is normalized through the
  top-level CLI
- non-interactive delegated CLI paths use the canonical `--json` contract
- dispatch-seam logging writes delegated dispatch-call and normalized-result
  entries for the active backend command
- delegated command paths still use the same documented top-level `command`,
  response, and error pattern as the rest of the CLI
- no backend crate gains a direct dependency on another backend crate

## Required Validation

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`

## Required Document Updates

- `docs/sc-lint/cli-requirements.md`
- `docs/sc-lint/cli-architecture.md`
- `docs/sc-lint/cli-contract.md`
- `docs/sc-lint/logging.md`
- `docs/architecture.md`
- `docs/project-plan.md`

## Risks And Watchouts

- do not let backend-specific flags become the public top-level contract
- do not turn config loading into duplicated wrapper behavior
- do not normalize only the happy path while leaving backend failures raw
