# Cross-Platform Guidelines

This document defines the high-level cross-platform expectations for `sc-lint`.

## Scope

`sc-lint` must run correctly on:

- Linux
- macOS
- Windows

This applies to:

- Rust crates
- repo-local lint runner automation
- CI workflows
- release packaging where applicable

## Requirements

- Prefer platform-neutral path handling.
- Do not assume Unix-only filesystem roots or separators.
- Keep shell and scripting behavior compatible with the repo CI matrix.
- Treat Windows support as first-class, not best-effort.

## Enforcement

- CI must continue to run on:
  - `ubuntu-latest`
  - `macos-latest`
  - `windows-latest`
- Portability rules in `sc-lint-portability` should continue to evolve as a
  separate lint family.

## PORT-008 Environment Variables

- `PORT-008` flags `HOME`, `USER`, and `XDG_*` lookups in ungated production
  code.
- The canonical detector is
  `production_env_portability_variable(...)` in
  `crates/sc-lint-portability/src/predicates.rs`, which matches `HOME`,
  `USER`, and `XDG_*` lookups inline.
- Preferred alternatives are `dirs::data_dir()`, `dirs::config_dir()`, and
  `dirs::home_dir()`.
- If the lookup is intentionally Unix-only, wrap the production code path in
  `#[cfg(unix)]` instead of leaving the env lookup ungated.

## PORT-009 Shell Invocation

- `PORT-009` flags production shell assumptions through `Command::new("sh")`,
  `Command::new("bash")`, and Unix shell path literals like `/bin/sh` or
  `/bin/bash`.
- The canonical detector sources are `UNIX_SHELL_COMMANDS` and
  `UNIX_SHELL_PATHS` in `crates/sc-lint-portability/src/predicates.rs`.
- Prefer direct binary invocation or a platform-neutral command abstraction
  instead of assuming a Unix shell exists.
- If shell execution is intentionally Unix-only, gate that production path with
  `#[cfg(unix)]`.

## PORT-010 `cfg` Parity

- `PORT-010` flags production `#[cfg(unix)]` items and impl methods when they
  do not have either a Windows companion or a portable ungated fallback.
- The canonical parity helpers live in
  `crates/sc-lint-portability/src/predicates.rs`, including
  `is_cfg_unix_production_item(...)`, `has_windows_companion(...)`,
  `has_portable_fallback(...)`, and the impl-method companion checks.
- `#[cfg(test)]` siblings do not satisfy production parity; companions must be
  production-facing.
- If no Windows companion exists, provide an ungated portable implementation or
  an explicit Windows-gated companion instead of leaving the Unix branch alone.

## Related Docs

- [docs/architecture.md](./architecture.md)
- [docs/project-plan.md](./project-plan.md)
