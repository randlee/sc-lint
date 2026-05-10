# `sc-lint` Structured Logging Plan

This document defines the planned structured logging integration for
`sc-lint`.

Related ADRs:
- [ADR-008 — `sc-observability` For `sc-lint` Structured Logging](./adr/ADR-008-sc-observability-logging.md)

## Goal

Use `sc-observability` as the logging-only runtime for the top-level CLI and
its delegated backend calls without turning logging initialization into a
backend-owned concern. Satisfies `REQ-LOG-001` and `REQ-LOG-005`.

This document defines both the logging design and the sprint-level
implementation assignments for Phase `A`. The current task is documentation
only; the Rust implementation lands in the owning Phase `A` sprints.

## Dependency Model

Requirement coverage:

- `REQ-LOG-001`
- `REQ-LOG-003`

The planned dependency is:

- crate:
  - `sc-observability`
- source:
  - local workspace path dependency declared in `Cargo.toml`
  - expected local checkout layout:
    - `../sc-observability/crates/sc-observability`
- integration mode:
  - path dependency from the `sc-lint` crate during local workspace
    development

The design depends on the following public surface:

- `LoggerBuilder`
- `LoggerConfig`
- `JsonlFileSink`
- `ConsoleSink`
- `ServiceName`

## Initialization Model

Requirement coverage:

- `REQ-LOG-001`
- `REQ-LOG-005`

Logging is a CLI-layer responsibility.

- the top-level `sc-lint` CLI initializes the logger at process startup
- backend crates must not initialize their own logger runtimes
- delegated backend execution is logged by the CLI around dispatch and result
  normalization
- direct library backends, when later added, receive logging through the same
  CLI-owned runtime rather than constructing a second logger
- subprocess backends, including Python tools still routed through Sprint
  `A.3`, run in separate processes and are not governed by the CLI logger
  runtime; their stdout/stderr handling is a separate dispatch concern

This keeps one process-wide logging authority for each `sc-lint` binary
invocation.

### Initialization Failure Contract

`LoggerBuilder` initialization failures must be surfaced as `CliError`
results before command dispatch proceeds.

Failure mode inventory:

- invalid or contradictory logging configuration
  - emit `CliError`
  - top-level kind/code:
    - `config`
    - `CLI.CONFIG_ERROR`
- resolved log path cannot be created or written in the current environment
  - emit `CliError`
  - top-level kind/code:
    - `capability`
    - `CLI.CAPABILITY_ERROR`
- unexpected builder or sink wiring failure inside the CLI process
  - emit `CliError`
  - top-level kind/code:
    - `internal`
    - `CLI.INTERNAL_ERROR`

The failure envelope should include recovery-oriented guidance because logging
startup is part of the top-level CLI contract rather than a backend-local
implementation detail.

## Service Names

Requirement coverage:

- `REQ-LOG-002`
- `REQ-LOG-004`

The planned service names are:

- `sc-lint`
  - top-level command orchestration
- `sc-boundary`
  - boundary analyzer command paths
- `sc-portability`
  - portability analyzer command paths
- `sc-runtime`
  - runtime analyzer command paths

The CLI chooses the service name from the active command path before
initializing the runtime for that invocation.

## Log Root Model

Requirement coverage:

- `REQ-LOG-002`

Default service-scoped log root:

- `~/sc-lint/logs/<service-name>/`

Planned override surfaces:

- config key:
  - `logging.root`
- CLI flag:
  - `--log-root <path>`

The override is per lint system because the service name is part of the
resolved root selection.

## Sink Model

Requirement coverage:

- `REQ-LOG-003`

### File Sink

File logging is on by default.

`sc-lint` should use `LoggerBuilder` rather than `Logger::new(...)` for the
default path, because the desired service-scoped directory layout is:

- `~/sc-lint/logs/<service-name>/`

while `LoggerConfig::default_for(...)` plus the built-in sink would otherwise
resolve the active file path relative to the provided root as:

- `<log_root>/logs/<service>.log.jsonl`

The planned integration is therefore:

1. build `LoggerConfig`
2. disable the built-in file sink
3. register one `JsonlFileSink` explicitly at a service-scoped path inside the
   chosen root directory

This preserves the requested directory model without requiring backend-local
logger setup.

### Console Sink

Console logging is opt-in.

Planned controls:

- CLI flag:
  - `--log-console`
- config key:
  - `logging.console`

When enabled, the CLI registers `ConsoleSink` through `LoggerBuilder`.

## Event Model

Requirement coverage:

- `REQ-LOG-004`

Every CLI invocation should emit:

1. invocation entry event
   - command
     - uses the same stable dotted command identifier documented for
       `CommandEnvelope.command`
   - effective settings/config used for the call
   - resolved args
   - timestamp
   - service name
2. completion event
   - verdict
   - summary
   - elapsed time in ms
   - service name
3. one error event per emitted top-level error
   - stable error code
   - `CliError.kind`
   - failure category
   - summary message

For delegated backends, the CLI also logs:

- dispatch start
- normalized result receipt after `CommandEnvelope<T>` / `CliError` mapping
- finding count when the backend returns findings payloads

## Rollout By Sprint

Requirement coverage:

- `REQ-LOG-001`
- `REQ-LOG-002`
- `REQ-LOG-003`
- `REQ-LOG-004`
- `REQ-LOG-005`

- `A.1a`
  - add the `sc-observability` dependency planning record
  - initialize the CLI-owned logger
  - emit top-level invocation, completion, and error events
- `A.1b`
  - log backend dispatch calls
  - log normalized delegated results
- `A.4`
  - add `sc-portability` analyzer entry/exit/finding-count logging to the
    delegated backend pattern
- `A.5`
  - apply the same analyzer logging pattern to `sc-runtime`
- `A.6`
  - add boundary-inventory loader entry/exit/error logging to the
    `sc-boundary` tool path
- `A.7`
  - add manifest-policy entry/exit/error logging to the `sc-boundary` tool
    path during the parity window
- `A.8`
  - document how users read command, verdict, elapsed-time, and stable-error
    log fields in the per-tool guides

## Ownership Rule

Requirement coverage:

- `REQ-LOG-005`

- the CLI owns logger initialization
- backend crates may emit through CLI-owned logging hooks later, but must not
  construct their own logger runtime

That rule is part of the product requirements and must remain true even if
some backends later move from delegated execution to direct library linkage.
