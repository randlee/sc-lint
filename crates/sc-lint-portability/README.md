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
- `PORT-006`
  - hardcoded Unix-only absolute path literals in production code
- `PORT-007`
  - hardcoded Windows absolute path literals, including drive-letter and UNC
    forms, in production code
- `PORT-008`
  - direct production lookups of `HOME`, `USER`, and `XDG_*` without a
    platform-neutral abstraction
- `PORT-009`
  - production `Command::new("sh" | "bash")` or hardcoded `/bin/sh` /
    `/bin/bash` shell-invocation assumptions without an explicit Unix-only
    boundary
- `PORT-010`
  - production `#[cfg(unix)]` items without a `#[cfg(windows)]` companion or
    explicit portable fallback in the same scope

## Ownership And Scope

Per [ADR-010](../../docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md),
`sc-lint-portability` is the shared product owner for consumer-neutral:

- Windows-path parity companion rules for the current Unix-only path checks
- broader environment-variable portability rules
- shell-portability rules for OS-specific shell-path and shell-command
  assumptions
- structural `cfg` parity rules for Unix-only production branches

Consumer-specific portability wrappers may still exist in downstream repos, but
they do not replace `sc-lint-portability` as the core shared ownership
boundary for these rule families.

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

Production code with a hardcoded Unix-only runtime path is also flagged:

```text
PORT-006 hardcoded Unix-only absolute path literal `/var/run/sc-lint` in production code; prefer dirs::cache_dir(), dirs::config_dir(), std::env::temp_dir(), or a platform-gated path abstraction
```

Production code with a hardcoded Windows runtime path is also flagged:

```text
PORT-007 hardcoded Windows-only absolute path literal `C:\ProgramData\sc-lint\cache.json` in production code; prefer dirs::cache_dir(), dirs::config_dir(), or a platform-gated path abstraction
```

Production code with Unix-centric environment lookups is also flagged:

```text
PORT-008 direct std::env lookup of `HOME` in production code bypasses platform-neutral path or identity abstractions; prefer dirs::data_dir(), dirs::config_dir(), dirs::home_dir(), or another platform-aware wrapper
```

Production code with Unix-shell assumptions is also flagged:

```text
PORT-009 Unix shell invocation `Command::new("sh" | "bash")` in production code assumes a Unix shell exists; prefer invoking the target binary directly or move the shell path behind an explicit Unix-only boundary
```

Production code with a Unix-only branch and no companion is also flagged:

```text
PORT-010 production #[cfg(unix)] item `runtime_socket_name` has no #[cfg(windows)] companion or explicit portable fallback in the same scope
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

- [logging.md](../../docs/sc-lint/logging.md)
- [cli-contract.md](../../docs/sc-lint/cli-contract.md)
