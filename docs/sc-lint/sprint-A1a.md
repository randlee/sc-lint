# Sprint A.1a — Top-Level CLI Bootstrap And Contract Definition

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.1a"
worktree: /Users/randlee/Documents/github/sc-lint
branch: develop
status: planned
estimated_scope: S
```

## Goal

Create the initial `sc-lint` crate as the stable top-level entry point and
define the canonical machine-readable success/failure contract before backend
integration begins.

## Scope Summary

This sprint establishes the CLI crate, typed command surface, and immutable
contract seam. It does not finish config loading or backend delegation, but it
must leave the product with a real top-level binary and a vetted
`CommandEnvelope<T>` / `CliError` contract.

## Governing Requirements

- `REQ-PRODUCT-001`
- `REQ-PRODUCT-002`
- `REQ-PRODUCT-002A`
- `REQ-PRODUCT-002B`
- `REQ-PRODUCT-002C`
- `REQ-PRODUCT-002D`
- `REQ-PRODUCT-002DA`
- `REQ-PRODUCT-002E`
- `REQ-PRODUCT-012D`
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

## Governing ADRs

- `docs/sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md`
- `docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md`

## Governing Boundaries

- `BOUNDARY-ScLintCli`
- `BOUNDARY-DirectiveModel`
- `BOUNDARY-ScLintBoundaryAnalyzer`

## Prerequisites

- canonical `sc-lint` boundary records and `boundaries/planning.toml` exist
- self-hosting `just lint` passes on the current workspace
- CLI contract docs are QA-clear on `develop`

## Hard Dependencies

- do not start config loading before the top-level command root exists
- do not start backend dispatch before the machine contract types are defined
- use this sprint to review the contract against the needs of Workstreams 4-7
  before locking the first delegated backend path

## Non-Goals

- implementing delegated backend commands
- adding `xwin` support
- migrating Python boundary logic into Rust
- changing backend crate ownership or introducing backend cross-dependencies

## Primary Targets

- `Cargo.toml`
- `crates/sc-lint/`
- `docs/sc-lint/cli-requirements.md`
- `docs/sc-lint/cli-architecture.md`
- `docs/sc-lint/cli-contract.md`
- `docs/project-plan.md`
- `boundaries/sc-lint/top-level-cli.toml`
- `boundaries/planning.toml`

## Sub-Tasks

1. Create the `sc-lint` crate and command root
   Development work:
   - add the `sc-lint` crate to the workspace
   - implement the top-level `Cli` command root and initial grouped command
     families
   - reserve the initial surface:
     - `lint`
     - `view`
     - `check`
     - `clippy`
     - `version`
     - `ci`
   Required tests:
   - command-parse tests for the initial grouped surface
   - help/usage tests for the top-level binary
   Required doc or boundary updates:
   - update crate inventory references if file/module names differ from the
     planned boundary records

2. Implement the canonical machine-readable contract
   Development work:
   - define `CommandEnvelope<T>` and `CliError`
   - implement canonical `--json` success and failure rendering
   - normalize exit-code behavior for top-level CLI-owned failures
   Required tests:
   - success-envelope serialization tests
   - failure-envelope serialization tests
   - exit-code tests for usage/config/internal failures
   Required doc or boundary updates:
   - keep `docs/sc-lint/cli-contract.md` aligned with the implemented field
     names and error-code families

3. Complete the A.1a contract-review checkpoint
   Development work:
   - review the planned CLI envelope against the needs of Workstreams 4-7
   - record any required pre-A.1b scope adjustments in the phase and sprint
     plan docs
   Required tests:
   - cross-doc review proving the contract checkpoint is assigned and
     sequenced before delegated backend work
   Required doc or boundary updates:
   - keep `docs/project-plan.md` and
     `docs/sc-lint/foundation-phase-plan.md` aligned with the A.1a/A.1b gate

## Split Recommendation

Keep A.1a together. The crate bootstrap and machine contract definition should
land in one reviewable unit before config loading and delegated backend
normalization begin.

## Acceptance Criteria

- the workspace contains a real `sc-lint` crate
- `sc-lint --help` exposes the initial grouped command surface
- non-interactive CLI-owned paths support canonical `--json`
- top-level machine-readable success uses `CommandEnvelope<T>`
- top-level machine-readable failure uses `CliError`
- the contract-review checkpoint for A.1b entry is documented and complete
- no backend crate gains a direct dependency on another backend crate

## Required Validation

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`

## Required Document Updates

- `docs/sc-lint/cli-requirements.md`
- `docs/sc-lint/cli-architecture.md`
- `docs/sc-lint/cli-contract.md`
- `docs/project-plan.md`
- `boundaries/sc-lint/top-level-cli.toml`
- `boundaries/planning.toml`

## Risks And Watchouts

- do not let backend-native JSON leak through as the public top-level
  contract
- do not make `--format json` part of the top-level CLI surface
- do not overfit the envelope to only the first delegated backend
