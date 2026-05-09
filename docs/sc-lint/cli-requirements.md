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

- `REQ-CLI-005A`
  Every non-interactive CLI command must support the canonical machine-readable
  mode:
  - `--json`

- `REQ-CLI-005B`
  Machine-readable mode must use stable success and failure contract families
  rather than falling back to prose-only stderr on top-level failures.

- `REQ-CLI-005C`
  Machine-readable failures must include stable error codes or categories,
  structured details, and caller-corrective guidance where recovery is
  possible.

- `REQ-CLI-005D`
  Human-readable output must not expose machine-significant information that is
  unavailable through `--json`.

- `REQ-CLI-005E`
  The CLI must document and preserve stable request and response models for
  command families exposed through the top-level machine contract.

- `REQ-CLI-005F`
  The CLI must define one stable top-level success envelope family for
  machine-readable results, even when delegated backends expose their own
  native machine payloads.

- `REQ-CLI-005G`
  The CLI must define one stable top-level failure envelope family using
  `CliError`, and that envelope must remain valid for:
  - direct library failures
  - delegated backend execution failures
  - delegated backend protocol/parse failures

- `REQ-CLI-006`
  The CLI must support both direct Rust-library dispatch and delegated
  subprocess-based execution during migration periods.

- `REQ-CLI-006A`
  During migration, the top-level CLI may translate canonical `--json`
  requests into backend-specific machine-output flags such as `--format json`,
  but backend-specific flag shapes must not become part of the stable
  top-level contract.

## Command Surface Requirements

- `REQ-CLI-007`
  The initial command surface must include:
  - `sc-lint lint <tool>`
  - `sc-lint view <tool>`
  - `sc-lint check`
  - `sc-lint clippy`
  - `sc-lint version`

- `REQ-CLI-007A`
  The CLI should support first-class execution modes for common developer
  flows where that yields a clearer contract than burying them inside generic
  lint target names.

- `REQ-CLI-007B`
  If `cargo xwin` is installed, the CLI should expose Windows preflight
  commands directly, with the initial intended shape:
  - `sc-lint check xwin`
  - `sc-lint clippy xwin`

- `REQ-CLI-007C`
  The CLI should support named lint profiles rather than requiring callers to
  reconstruct profile membership from individual tools.

- `REQ-CLI-007D`
  The initial lint profile names should be:
  - `sc-lint lint fast`
  - `sc-lint lint full`
  - `sc-lint lint ci`

- `REQ-CLI-007E`
  The CLI should provide a top-level CI-equivalent command:
  - `sc-lint ci`
  which includes tests, while `sc-lint lint ci` remains lint-only.

- `REQ-CLI-008`
  The CLI must preserve room for additional grouped subcommands without
  breaking the initial shape.

## Planned Contract Types

- `REQ-CLI-008A`
  The planned CLI contract must explicitly define:
  - `Cli`
  - `Command`
  - `LintProfile`
  - `OutputMode`
  - `CliError`

- `REQ-CLI-008B`
  `LintProfile` must define the product-level profile values:
  - `Fast`
  - `Full`
  - `Ci`

- `REQ-CLI-008C`
  `OutputMode` must define:
  - `Human`
  - `Json`

- `REQ-CLI-008D`
  `CliError` must be documented as a structured machine-readable contract with
  at least:
  - error kind or category
  - stable code
  - message
  - optional details
  - optional suggested action

- `REQ-CLI-008E`
  Future interactive graph-exploration surfaces must not replace the
  documented `OutputMode::Json` contract for machine use.

- `REQ-CLI-008F`
  `CommandEnvelope<T>` must be documented as the top-level machine-readable
  result family for non-interactive command execution.

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

- `REQ-CLI-014`
  If `cargo xwin` is installed, `xwin`-backed Windows preflight should join
  the `fast` and `full` lint profiles where the specific command remains fast
  enough for that profile.

- `REQ-CLI-015`
  `xwin`-backed preflight must not be part of the `ci` lint profile because
  the product relies on real Windows CI runners for authoritative validation.

## Contract References

- See [cli-contract.md](./cli-contract.md) for:
  - top-level success envelope
  - `CliError` failure envelope
  - backend-to-CLI normalization rules
  - exit-code mapping guidance
