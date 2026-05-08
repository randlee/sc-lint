# sc-lint CLI Architecture

This document records the intended architecture of the top-level `sc-lint` CLI
crate.

## Role

The `sc-lint` CLI is the orchestration layer for the tool family.

It owns:

- command parsing
- config loading
- output/exit-code normalization
- dispatch to backend tools

It does not own:

- deep backend analysis logic
- backend crate business rules

## Architectural Model

The intended model is:

```text
sc-lint (CLI)
  -> Rust library backend, when available
  -> Rust binary backend, when needed
  -> Python utility, during migration windows
```

## Dispatch Principles

- backend crates remain self-contained
- the CLI decides which backend is used
- backend crates do not call each other directly
- backend replacement should not require changing the CLI command contract

## Initial Command Families

- `lint`
  - backend lint tools and wrappers
- `view`
  - report and visualization tools
- `version`
  - version and upgrade inspection
- `ci`
  - repo CI-equivalent orchestration including tests

Planned direct platform-aware command family:

- `check`
  - native or cross-target compile checks
- `clippy`
  - native or cross-target clippy runs

Initial `xwin`-aware command direction:

- `sc-lint check xwin`
- `sc-lint clippy xwin`

Planned initial lint profiles:

- `sc-lint lint fast`
- `sc-lint lint full`
- `sc-lint lint ci`

Planned top-level CI-equivalent command:

- `sc-lint ci`

Profile semantics:

- `fast`
  - low-latency local developer gate
  - includes `xwin` checks only when the individual command is fast enough
- `full`
  - stronger local pre-push gate
  - may include slower `xwin` checks such as `clippy xwin`
- `ci`
  - lint-only profile aligned to what the project considers CI lint parity
  - does not include `xwin`
- top-level `ci`
  - lint plus tests
  - mirrors real CI intent rather than `xwin` preflight

## Config Flow

Expected flow:

1. discover repo root
2. load shared config
3. resolve subcommand/tool target and capability requirements
4. dispatch to backend
5. normalize output and exit code

For `xwin`-aware commands, capability resolution includes:

- detect whether `cargo xwin` is installed
- select the supported Windows target set
- add `xwin`-aware checks into `fast` or `full` only when the capability is
  present
- keep `ci` profile semantics independent from `xwin`
- skip or error with a clear capability message depending on command mode

## Output Model

The CLI should present:

- consistent human-readable text output
- stable machine-readable output modes where supported
- stable success/failure exit codes across delegated tools

## Migration Role

The CLI is specifically intended to let `sc-lint` evolve without forcing all
tools into Rust at once.

That means it should tolerate:

- Rust-native tools
- Python-backed tools
- future extracted binaries

behind one stable user-facing surface.
