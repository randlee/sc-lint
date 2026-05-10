# `sc-lint` Structured Logging Plan

This document defines the structured logging integration for
`sc-lint`.

Related ADRs:
- [ADR-008 — `sc-observability` For `sc-lint` Structured Logging](./adr/ADR-008-sc-observability-logging.md)

## Goal

Use `sc-observability` as the logging-only runtime for the top-level CLI and
its delegated backend calls without turning logging initialization into a
backend-owned concern. Satisfies `REQ-LOG-001` and `REQ-LOG-005`.

This document defines both the logging design and the sprint-level
implementation assignments for Phase `A`. The A.1a bootstrap implementation is
now present in `crates/sc-lint/src/logging.rs`; A.1b extends that same
CLI-owned runtime with dispatch-seam logging for the first real backend path.

## Dependency Model

Requirement coverage:

- `REQ-LOG-001`
- `REQ-LOG-003`

The planned dependency is:

- crate:
  - `sc-observability`
- source:
  - published Cargo dependency resolved through crates.io
- integration mode:
  - regular workspace dependency from the `sc-lint` crate so CI runners and
    local development use the same source of truth

The design depends on the following public surface:

- `LoggerBuilder`
- `LoggerConfig`
- `JsonlFileSink`
- `ConsoleSink`
- `ServiceName`

Planned implementation note:

- construct `ServiceName` once in the top-level CLI command-resolution layer
  from the stable dotted `command` identifier
- pass that validated service identity into logger setup and event emission
  helpers rather than rebuilding raw strings at each call site
- the A.1a bootstrap currently maps:
  - `version`
    - `sc-lint`
  - `lint.sc-boundary`
    - `sc-boundary`
  - `lint.sc-portability`
    - `sc-portability`
  - `lint.sc-runtime`
    - `sc-runtime`
  - remaining bootstrap-only command paths
    - `sc-lint`

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

Planned implementation note:

- represent the validated effective log root as a dedicated `LogRoot`
  wrapper/newtype at the CLI config boundary rather than passing a raw
  `String` through multiple modules
- the A.1a implementation applies the service name to the resolved root so
  `--log-root <path>` becomes:
  - `<path>/<service-name>/`

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

When enabled, the A.1a bootstrap turns on
`LoggerConfig.enable_console_sink` before `LoggerBuilder::new(...)`. Later
sprints should preserve the same CLI surface unless explicit per-sink
filtering becomes necessary.

## Concurrency Model

The logging runtime is process-wide for one `sc-lint` invocation and must be
safe to share across all command-dispatch code that runs inside that process.

Planned constraints:

- the CLI owns construction of the runtime and sink set once at startup
- the constructed runtime handle must be shareable across in-process command
  execution without backend-local reinitialization
- backend crates must treat logging as an already-installed facility rather
  than a mutable global they control
- sink-thread-safety and cross-thread emission behavior must match the
  concrete `sc-observability` runtime surface chosen during implementation

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
   - emitted for both success and failure verdicts so the command always has a
     closed lifecycle record
3. one error event per emitted top-level error
   - stable error code
   - `CliError.kind`
   - failure category
   - summary message

Event field table:

| Event | Required fields | Python-adapter additions in A.3 |
| --- | --- | --- |
| `cli.command.started` | `command`, effective settings/config, resolved args, timestamp, service name | For `lint.line-counts`, `lint.identity-literals`, and `view.findings`, also include `adapter`, `config_scope`, and `script`. |
| `cli.command.completed` | `command`, `verdict`, `summary`, elapsed time in ms, service name | Carry the same `adapter`, `config_scope`, and `script` fields when the completed command used the Python-adapter path. |
| `cli.command.error` | `command`, stable error code, `CliError.kind`, failure category, summary message | Carry the same `adapter`, `config_scope`, and `script` fields when the emitted `CliError` came from a Python-adapter command path. |

For delegated backends, the CLI also logs:

- dispatch start
- normalized result receipt after `CommandEnvelope<T>` / `CliError` mapping
- finding count when the backend returns findings payloads

Dispatch failure contract:

- if dispatch fails after the entry event but before a successful backend
  payload is normalized, the CLI must emit:
  - one `CliError`-backed error event
  - one completion event with a failure verdict and elapsed time in ms
- the CLI must not leave a started command path without either a completion
  event or a top-level failure envelope

The A.1a bootstrap action names are:

- `cli.command.started`
- `cli.command.completed`
- `cli.command.error`

The A.1b dispatch action names are:

- `cli.dispatch.started`
- `cli.dispatch.normalized`

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
  - load `logging.root` and `logging.console` through the top-level config
    loader before initializing the runtime
- `A.2`
  - `xwin` preflight now emits entry/exit/error logging through the standard
    CLI event pattern
  - `check.xwin` and `clippy.xwin` log `preflight_mode=xwin` and the effective
    Windows target triple in the CLI-owned event fields
- `A.3`
  - add Python utility entry/exit/error logging through the adapter-normalized
    CLI event pattern
  - include the Python-adapter metadata fields:
    - `adapter`
    - `config_scope`
    - `script`
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
