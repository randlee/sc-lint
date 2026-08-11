---
id: E.1
title: Compatibility Contract And Version Preflight
status: planned
branch: feature/phase-E1-compatibility-contract
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/phase-E1-compatibility-contract
target: integrate/phase-E
---

# Sprint E.1 — Compatibility Contract And Version Preflight

## Goal

Define one typed, machine-readable contract for a repository's minimum
`sc-lint` version and for evaluating the system installation before consumer
commands run.

## Governing Plan

- [Phase E plan](./phase-E-plan.md)

## Hard Dependencies

- `crates/sc-lint/src/command.rs` current version command/JSON envelope
- `crates/sc-lint/src/config.rs` configuration discovery
- `crates/sc-lint/src/contract.rs` and `CliError` error envelope
- `sc-lint.toml` and `.just/lint-config.toml` current configuration behavior

## Exact Targets

- `crates/sc-lint/src/config.rs`
- `crates/sc-lint/src/command.rs`
- `crates/sc-lint/src/cli.rs`
- `crates/sc-lint/src/contract.rs`
- `crates/sc-lint/src/tests.rs`
- `docs/requirements.md`
- `docs/sc-lint/cli-requirements.md`
- `docs/sc-lint/cli-contract.md`
- `docs/phase-E/sprint-E1.md`

## Deliverables

- `sc-lint.toml` accepts exactly one canonical requirement location:

  ```toml
  [tool.sc-lint]
  minimum_version = "0.4.1"
  ```

- `minimum_version` is parsed once into a validated semantic-version newtype;
  no later caller compares raw version strings. This is `RBP-004` applied to a
  public compatibility boundary.
- `sc-lint --json version` provides the stable non-repository version probe.
  It includes tool name, SemVer version, and contract schema version, and
  writes no report/log files. The existing global `--json` spelling is retained
  instead of creating a second output-mode grammar.
- a check-oriented command/API evaluates the installed binary against the
  loaded repository requirement without executing any lint/test work.
- error cases use stable error codes and include recovery guidance:
  - configuration missing or malformed
  - binary not found
  - installed version unparsable
  - installed version lower than the configured floor
  - required binary cannot be executed
- the human and JSON forms name the required version, observed version/path
  when available, and the supported `just setup` / installer recovery action.
- `docs/requirements.md` gains normative requirements for: one tracked
  minimum-version field, SemVer comparison, a stable non-repository version
  probe, and structured missing/incompatible-install recovery. The CLI
  requirements and contract document map each requirement to the command and
  error envelopes in this sprint.

## Explicit Contract Sample

```json
{
  "tool": "sc-lint",
  "version": "0.4.1",
  "contract_schema": "sc-lint-version-v1",
  "status": "pass"
}
```

```json
{
  "error": {
    "code": "CLI.SC_LINT_VERSION_TOO_OLD",
    "message": "installed sc-lint 0.4.0 does not satisfy minimum version 0.4.1",
    "cause": "repository requires a newer sc-lint capability set",
    "recovery": "run `just setup` to install or upgrade sc-lint",
    "docs": "sc-lint docs setup"
  }
}
```

## Acceptance Criteria

- SemVer comparisons cover equal, higher, lower, prerelease, malformed, and
  missing values without lexical string comparison.
- version probing works outside a Cargo workspace and does not require a
  repository root.
- a malformed `minimum_version` identifies the configuration file and field.
- every failing path satisfies `RBP-001`: stable code, cause, recovery, and
  documentation reference.
- requirements, CLI requirements, and CLI contract updates are reviewed in
  the same change as the command/configuration implementation.
- unit tests cover all result states and both human/JSON output contracts.

## Required Validation

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p sc-lint`
- contract tests run the released-style version probe outside a workspace
