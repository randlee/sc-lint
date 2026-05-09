# Sprint A.1 — Top-Level CLI Bootstrap

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.1"
worktree: /Users/randlee/Documents/github/sc-lint
branch: develop
status: planned
estimated_scope: M
```

## Goal

Create the initial `sc-lint` crate as the stable top-level entry point with
command parsing, repo-config loading, and the canonical machine-readable
success/failure contract.

## Scope Summary

This sprint establishes the CLI crate and its contract seam. It does not
finish every backend integration, but it must leave the product with a real
top-level binary, typed command surface, and one normalized `--json` contract
for non-interactive commands.

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
- `REQ-CLI-008A`
- `REQ-CLI-008B`
- `REQ-CLI-008C`
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

- do not start profile/xwin work before the top-level command root and machine
  contract types exist
- do not start Python-tool dispatch before one canonical top-level envelope is
  implemented and tested

## Non-Goals

- implementing every delegated backend command
- adding `xwin` support
- migrating Python boundary logic into Rust
- changing backend crate ownership or introducing backend cross-dependencies

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

2. Implement top-level config loading
   Development work:
   - define the repo-root discovery path
   - implement one top-level config loader for CLI-owned commands
   - keep backend-specific config parsing behind the CLI contract seam
   Required tests:
   - repo-root discovery tests
   - malformed-config tests
   Required doc or boundary updates:
   - update CLI architecture docs if the config entry point name changes

3. Implement the canonical machine-readable contract
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

4. Implement first backend dispatch seam
   Development work:
   - add one real delegated command path through the CLI, preferably
     `sc-lint lint sc-boundary`
   - normalize backend machine output into the top-level envelope
   - handle backend protocol and execution failures explicitly
   Required tests:
   - delegated success-path tests
   - malformed backend JSON tests
   - backend execution failure tests
   Required doc or boundary updates:
   - update CLI contract docs if the normalization rules need narrower wording

## Split Recommendation

Keep A.1 together. The crate bootstrap, machine contract, and first backend
dispatch seam should land in the same sprint so the product exits A.1 with a
real top-level CLI rather than another paper design.

## Acceptance Criteria

- the workspace contains a real `sc-lint` crate
- `sc-lint --help` exposes the initial grouped command surface
- non-interactive CLI paths support canonical `--json`
- top-level machine-readable success uses `CommandEnvelope<T>`
- top-level machine-readable failure uses `CliError`
- at least one real delegated backend command is normalized through the
  top-level CLI
- no backend crate gains a direct dependency on another backend crate

## Required Validation

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`

## Required Document Updates

- `docs/sc-lint/cli-requirements.md`
- `docs/sc-lint/cli-architecture.md`
- `docs/sc-lint/cli-contract.md`
- `docs/architecture.md`
- `docs/project-plan.md`
- `boundaries/sc-lint/top-level-cli.toml`
- `boundaries/planning.toml`

## Risks And Watchouts

- do not let backend-native JSON leak through as the public top-level
  contract
- do not make `--format json` part of the top-level CLI surface
- do not turn the first CLI implementation into a thin shell with no typed
  success/failure normalization
