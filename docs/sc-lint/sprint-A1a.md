# Sprint A.1a — Top-Level CLI Bootstrap And Contract Definition

```yaml
plan_type: sprint_plan
phase: A
sprint: "A.1a"
worktree: <repo-root>
branch: develop
status: implemented
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

The broader `REQ-CLI-008A` contract inventory is only partially satisfied in
A.1a by design. A.1a defines `Cli`, `Command`, `CommandEnvelope<T>`, and
`CliError`; `LintProfile` and `OutputMode` remain assigned to Sprint `A.2`
because they depend on the profile/preflight strategy gate.

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
- `docs/sc-lint/logging.md`
- `docs/requirements.md`
- `docs/project-plan.md`
- `boundaries/sc-lint/top-level-cli.toml`
- `boundaries/planning.toml`

## Sub-Tasks

1. Create the `sc-lint` crate and command root
   Development work:
   - add the `sc-lint` crate to the workspace
   - implement the top-level `Cli` command root and initial grouped command
     families
   - split responsibilities early into command-parsing, contract, error,
     rendering, and logging seams so later command families do not copy/paste
     their own output behavior
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
   - define the stable dotted `command` identifier convention for every
     initial non-interactive command family
   - keep envelope serialization and `CliError` mapping in one shared contract
     path rather than per-command handlers
   - normalize exit-code behavior for top-level CLI-owned failures
   Required tests:
   - success-envelope serialization tests
   - failure-envelope serialization tests
   - fixture or snapshot tests proving `lint`, `view`, `check`, `clippy`,
     `ci`, and `version` all use the same top-level envelope and failure keys
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

4. Plan structured logging bootstrap
   Development work:
   - add the `sc-observability` path dependency plan for the `sc-lint` crate
   - define top-level logger initialization ownership at CLI startup
   - define invocation entry, completion, and per-error event logging for
     top-level CLI commands
   Required tests:
   - doc review for service-name, log-root, and sink-policy consistency
   Required doc or boundary updates:
   - add `docs/sc-lint/logging.md`
   - update `docs/requirements.md`
   - update `boundaries/planning.toml`

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
- the initial non-interactive command families share one documented `command`,
  response, and error pattern at the top-level envelope
- the CLI logger initializes at process startup and writes invocation entry,
  completion, and per-error events under `~/sc-lint/logs/sc-lint/`
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
- `docs/requirements.md`
- `docs/project-plan.md`
- `boundaries/sc-lint/top-level-cli.toml`
- `boundaries/planning.toml`

## Risks And Watchouts

- do not let backend-native JSON leak through as the public top-level
  contract
- do not make `--format json` part of the top-level CLI surface
- do not overfit the envelope to only the first delegated backend

## Implemented A.1a Surface

The implemented `sc-lint` crate lands the following bootstrap seams:

- `crates/sc-lint/src/cli.rs`
  - top-level `Cli` plus grouped command parsing for:
    - `lint`
    - `view`
    - `check`
    - `clippy`
    - `version`
    - `ci`
- `crates/sc-lint/src/command.rs`
  - canonical dotted `command` identifiers
  - service-name selection for structured logging
  - reserved command handling for not-yet-integrated command families
- `crates/sc-lint/src/contract.rs`
  - `CommandEnvelope<T>`
- `crates/sc-lint/src/error.rs`
  - `CliError`
  - stable top-level error-code mapping
- `crates/sc-lint/src/render.rs`
  - canonical `--json` rendering
  - human-readable rendering derived from the same normalized result
- `crates/sc-lint/src/logging.rs`
  - CLI-owned logger bootstrap
  - entry/completion/error event emission

The A.1a reserved command inventory is:

- `sc-lint lint sc-boundary`
- `sc-lint lint sc-portability`
- `sc-lint lint sc-runtime`
- `sc-lint lint fast`
- `sc-lint lint full`
- `sc-lint lint ci`
- `sc-lint view graph`
- `sc-lint view findings`
- `sc-lint check native`
- `sc-lint check xwin`
- `sc-lint clippy native`
- `sc-lint clippy xwin`
- `sc-lint ci`

In A.1a those reserved surfaces intentionally return top-level
`CLI.CAPABILITY_ERROR` envelopes rather than ad hoc prose so later sprints can
add real execution paths without changing the contract family. The direct
success-path bootstrap command in A.1a is `sc-lint version`.

## A.1a Contract-Review Checkpoint

The A.1a exit review against Workstreams 4-7 is complete and is the gate for
A.1b entry.

### Workstream 4: Generic Python utility extraction

- `view` stays a reserved top-level grouping in A.1a.
- the documented bootstrap targets are:
  - `view.graph`
  - `view.findings`
- A.3 must normalize Python-backed utility output through the same
  `CommandEnvelope<T>` / `CliError` path rather than exposing raw Python
  stderr or per-tool JSON.

### Workstream 5: Boundary logic migration to Rust

- A.1b is approved to make `sc-lint lint sc-boundary` the first real backend
  path.
- A.1b must keep repo-root discovery and config loading in the top-level CLI
  before calling `sc-lint-boundary`.
- backend-native machine output must still normalize through one shared
  top-level envelope path.

### Workstream 6: `sc-lint-portability`

- the stable top-level target name is fixed as `lint.sc-portability`.
- A.4 may use delegated backend execution first; direct top-level crate
  linkage is not assumed by A.1a.
- the CLI-owned logger must continue to choose `sc-portability` as the
  service identity when that backend path becomes real.

### Workstream 7: `sc-lint-runtime`

- the stable top-level target name is fixed as `lint.sc-runtime`.
- A.5 must reuse the same contract and logging path rather than introducing a
  runtime-specific response envelope.
- any future direct-linked runtime backend remains subject to the CLI-owned
  logger invariant from ADR-008.
