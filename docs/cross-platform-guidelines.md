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

## PORT-006 Unix Path Literals

- `PORT-006` flags built-in Unix absolute path prefixes in ungated production
  code.
- Path literals in items and expression blocks gated by `#[cfg(unix)]`,
  `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, or
  `#[cfg(not(windows))]` are exempt. Equivalent nested `all(...)` and `any(...)` cfg lists are recognized
  when they contain one of those conditions.
- Statement-level `#[cfg]` attributes are not honored. This includes cfg
  attributes on `let` bindings, block/if expressions, and call expressions,
  consistent with ADR-020 decision 1.
- The same leaf gate suppresses PORT-007, PORT-008 and PORT-009 findings
  inside those platform-specific paths. It does not change PORT-004 or
  PORT-010 parity classification.
- The canonical detector is `attr_is_platform_cfg(...)` in
  `crates/sc-lint-portability/src/predicates.rs`.

## PORT-007 Windows Path Literals

- `PORT-007` flags drive-absolute and UNC path literals in ungated production
  code.
- Windows-gated items are exempt from the portability leaf findings,
  including drive and UNC path literals. Statement-level cfg attributes do
  not provide this exemption.
- Literals whose unescaped value starts with the Windows verbatim prefix
  `\\?\` or device prefix `\\.\` are not reported; they are namespace
  syntax, not absolute paths for this rule.
- Other UNC literals, including `\\server\share` and `\\?x`, remain in
  scope. No reviewed-exception allowlist is provided.
- The canonical detector is `is_windows_path_literal(...)` in
  `crates/sc-lint-portability/src/predicates.rs`.

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
- A `USER` read is not reported when the same enclosing function also reads
  `USERNAME` (including through a closure or nested item in that body).

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
- A private `#[cfg(unix)]` function or module, or a private inherent-impl
  method, in either file of a configured `platform_adapters` pair is exempt.
  `pub`, `pub(crate)`, `pub(super)`, `pub(in ..)` items and trait-impl methods
  in those files retain their parity checks.

## Related Docs

- [docs/architecture.md](./architecture.md)
- [docs/project-plan.md](./project-plan.md)
