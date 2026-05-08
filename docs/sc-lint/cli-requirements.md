# sc-lint CLI Requirements

This document defines the detailed requirements for the planned top-level
`sc-lint` CLI crate.

## Purpose

The CLI exists to provide one stable user-facing command surface across
specialized backend tools and mixed Rust/Python implementations.

## Functional Requirements

- `REQ-CLI-001`
  The CLI must provide a stable top-level executable named `sc-lint`.

- `REQ-CLI-002`
  The CLI must support subcommand-based command parsing.

- `REQ-CLI-003`
  The CLI must load repo config before backend dispatch.

- `REQ-CLI-004`
  The CLI must normalize exit-code behavior across delegated tools.

- `REQ-CLI-005`
  The CLI must normalize user-facing output conventions across delegated tools.

- `REQ-CLI-006`
  The CLI must support both direct Rust-library dispatch and delegated
  subprocess-based execution during migration periods.

## Command Surface Requirements

- `REQ-CLI-007`
  The initial command surface must include:
  - `sc-lint lint <tool>`
  - `sc-lint view <tool>`
  - `sc-lint version`

- `REQ-CLI-008`
  The CLI must preserve room for additional grouped subcommands without
  breaking the initial shape.

## Architecture Requirements

- `REQ-CLI-009`
  The CLI must not require specialized backend tool crates to depend on each
  other directly.

- `REQ-CLI-010`
  Backend coordination must happen in the CLI layer rather than by backend
  crate cross-calls.

- `REQ-CLI-011`
  Shared support crates may be introduced only after explicit design review.

## Migration Requirements

- `REQ-CLI-012`
  During extraction and migration, the CLI may dispatch to Python utilities for
  tools not yet ported to Rust.

- `REQ-CLI-013`
  Once Rust-native replacements exist, the CLI should be able to swap the
  backend implementation without changing the user-facing command contract.
