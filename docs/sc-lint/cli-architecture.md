# sc-lint CLI Architecture

This document records the intended architecture of the top-level `sc-lint` CLI
crate.

Related ADRs:
- [`./adr/ADR-005-cli-profiles-and-xwin-preflight.md`](./adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- [`./adr/ADR-006-ai-first-cli-contract.md`](./adr/ADR-006-ai-first-cli-contract.md)

## Role

The `sc-lint` CLI is the orchestration layer for the tool family.

It owns:

- command parsing
- config loading
- output/exit-code normalization
- dispatch to backend tools
- the canonical top-level machine-readable contract

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

The top-level CLI is not only a dispatcher. It is also the stable contract
owner for:

- top-level command grouping
- canonical machine-mode selection through `--json`
- shared profile names
- shared success/failure envelope conventions
- capability-aware dispatch such as `xwin`

For release `0.1.x`, the CLI may satisfy that role through a mix of:

- direct library linkage to stable support/backend crates
- delegated subprocess dispatch to backend binaries or Python adapters

The stable contract is the top-level envelope and command surface, not the
dispatch mechanism used behind it.

## Dispatch Principles

- backend crates remain self-contained
- the CLI decides which backend is used
- backend crates do not call each other directly
- backend replacement should not require changing the CLI command contract
- backend-specific machine flags must stay behind the CLI contract boundary

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

## Planned Contract Types

The release-1 CLI design should name and preserve the following important
types explicitly:

- `Cli`
  - top-level command root
- `Command`
  - grouped command family selector
- `LintProfile`
  - enum with:
    - `Fast`
    - `Full`
    - `Ci`
- `OutputMode`
  - enum with:
    - `Human`
    - `Json`
- `CommandEnvelope<T>`
  - top-level success/failure result family
- `CliError`
  - structured machine-readable error contract carrying:
    - error kind/category
    - stable code
    - message
    - optional details
    - optional suggested action

These names define the intended architectural seam even before all of them are
fully implemented.

For release `0.1.x`, these planned contract types should also be represented as
`BOUNDARY-ScLintCli` composition-root items in the boundary/planning metadata.

## Machine Contract Model

For non-interactive commands, `--json` is the canonical top-level machine
contract mode.

That means:

- every non-interactive command family must support machine-readable output
- top-level success and failure paths must stay inside one machine-contract
  family
- human-readable output is a presentation layer over the same underlying
  command result

During migration, the CLI may internally translate:

- top-level `--json`

into backend-specific flags such as:

- `--format json`

but that backend translation must remain an implementation detail.

## Contract Ownership

The top-level CLI owns:

- top-level command names
- profile names
- output-mode semantics
- top-level machine-readable envelope conventions
- normalization of delegated tool results into stable success/failure behavior

Backend tools own:

- family-specific request/response payloads
- analyzer-specific findings payloads
- domain-specific diagnostics beneath the CLI surface

Future MCP wrappers should reuse the same request and response models rather
than introducing a second business-payload schema.

## Backend Normalization Path

The end-to-end result path should be:

```text
backend-native success/error
  -> top-level CLI normalization
  -> CommandEnvelope<T> or CliError
  -> human rendering or canonical --json output
```

Required normalization cases:

- direct Rust-library backend error
  - mapped into `CliError`
- delegated Rust binary success
  - parsed and wrapped into `CommandEnvelope<T>`
- delegated Rust binary malformed machine output
  - mapped into `CLI.BACKEND_PROTOCOL_ERROR`
- delegated Rust binary execution failure without valid machine payload
  - mapped into `CLI.BACKEND_EXEC_FAILURE`
- delegated Python utility failure
  - normalized into `CliError` rather than exposing raw traceback text as the
    public machine contract

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
- stable machine-readable output for every non-interactive command
- stable success/failure exit codes across delegated tools
- stable machine-readable failure contracts in `--json` mode

See [cli-contract.md](./cli-contract.md) for the detailed envelope and
normalization contract, including the planned exit-code mapping.

## Interactive Constraint

Future graph exploration may justify interactive or TUI-oriented commands, but
those must remain secondary surfaces.

They must not:

- become the only way to obtain machine-significant data
- be richer than the documented machine-readable contract in a way that forces
  automation to parse TTY output

## Migration Role

The CLI is specifically intended to let `sc-lint` evolve without forcing all
tools into Rust at once.

That means it should tolerate:

- Rust-native tools
- Python-backed tools
- future extracted binaries

behind one stable user-facing surface.
