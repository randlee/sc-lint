# `sc-portability` User Guide

`sc-portability` is the shipped portability-analysis tool for cross-platform
Rust source checks.

## Purpose

Use `sc-portability` to catch portability drift such as:

- `PORT-001`
  - hardcoded Unix-only absolute paths in test code
- `PORT-002`
  - direct `dirs::home_dir()` without the configured override check
- `PORT-003`
  - `std::env::set_var()` in test code
- `PORT-004`
  - ungated `std::os::unix` imports in production code
- `PORT-005`
  - `#[cfg_attr(not(unix), allow(dead_code))]` portability suppressors

## Invocation

Primary product surface:

```bash
sc-lint lint sc-portability
sc-lint --json lint sc-portability
```

Repo-local convenience path:

```bash
just lint sc-portability
```

Backend-local path:

```bash
cargo run -p sc-lint-portability -- analyze --root . --format text
cargo run -p sc-lint-portability -- analyze --root . --format json
```

## Representative Pass

Production code with Unix-only imports correctly gated:

```rust
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn configure() {}
```

Representative text result:

```text
sc-lint-portability: PASS
scanned crates: 1
findings: 0
```

## Representative Fail

Production code with an ungated Unix-only import:

```rust
use std::os::unix::fs::PermissionsExt;

pub fn configure() {}
```

Representative finding:

```text
PORT-004 ungated std::os::unix import in production code; wrap the item with #[cfg(unix)] or move the import behind a Unix-only boundary
```

Test-only path literals are also flagged:

```text
PORT-001 hardcoded Unix-only absolute path `/tmp/example` in test code; prefer std::env::temp_dir(), dirs::home_dir(), or another platform-aware path source
```

## Disable Model

There is currently no approved rule-disable path in the shipped
`sc-lint lint sc-portability` surface.

Important limits:

- no top-level CLI rule-disable flags
- no backend-specific allowlist flags documented for release `0.1.x`
- repo-local Python lint directives are not the product contract for this Rust
  analyzer

If a rule needs policy exceptions later, that must be added explicitly in the
owning backend contract and documented there first.

## Output And Logging

Canonical machine mode:

```bash
sc-lint --json lint sc-portability
```

The top-level envelope is the same shared CLI contract:

- success:
  - `ok: true`
  - `command: "lint.sc-portability"`
  - findings payload under `data`
- failure:
  - `ok: false`
  - `command: "lint.sc-portability"`
  - stable `CliError` object under `error`

Structured logs for this tool use:

- service name:
  - `sc-portability`
- command identity:
  - `lint.sc-portability`

Read the command lifecycle through:

- `cli.command.started`
- `cli.command.completed`
  - includes `summary` and `elapsed_ms`
- `cli.command.error`
  - includes stable error code and `CliError.kind`

Delegated/backend normalization records also appear:

- `cli.dispatch.started`
- `cli.dispatch.normalized`
  - includes the delegated tool name and normalized finding count

See:

- [logging.md](../logging.md)
- [cli-contract.md](../cli-contract.md)

