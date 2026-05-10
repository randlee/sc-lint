# sc-lint CLI Contract

This document defines the planned end-to-end contract for the top-level
`sc-lint` CLI.

Related ADRs:
- [`./adr/ADR-005-cli-profiles-and-xwin-preflight.md`](./adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- [`./adr/ADR-006-ai-first-cli-contract.md`](./adr/ADR-006-ai-first-cli-contract.md)
- [`./adr/ADR-008-sc-observability-logging.md`](./adr/ADR-008-sc-observability-logging.md)

It exists to close the gap between:

- backend-native result shapes
- delegated tool execution
- the final user-facing CLI contract

## Purpose

The top-level `sc-lint` CLI is not only a dispatcher. It is also the canonical
machine-contract owner for non-interactive commands.

That means:

- `--json` is the canonical machine mode
- success and failure are both machine-readable in machine mode
- backend-specific flags and result shapes stay behind the CLI boundary

## Planned Contract Types

The release-1 contract should explicitly define these types:

- `Cli`
- `Command`
- `LintProfile`
- `OutputMode`
- `CommandEnvelope<T>`
- `CliError`

## Contract Invariants For Every Non-Interactive Command

Every non-interactive top-level command must preserve the same contract shape:

- success:
  - `ok: true`
  - stable `command`
  - family-specific payload under `data`
  - optional additive `diagnostics`
- failure:
  - `ok: false`
  - stable `command`
  - `CliError` under `error`
  - optional additive `diagnostics`

Commands must not introduce family-specific top-level envelope keys such as:

- `findings` at the top level for lint only
- `report` at the top level for view only
- `steps` at the top level for CI only

Those values belong under `data` so the top-level machine contract remains
consistent.

## Command Identity Convention

The `command` field is a stable dotted identifier derived from the final CLI
path selected by the caller.

Initial convention:

- `sc-lint lint sc-boundary`
  - `lint.sc-boundary`
- `sc-lint lint fast`
  - `lint.fast`
- `sc-lint view <target>`
  - `view.<target>`
- `sc-lint check xwin`
  - `check.xwin`
- `sc-lint clippy xwin`
  - `clippy.xwin`
- `sc-lint ci`
  - `ci`
- `sc-lint version`
  - `version`

The same identifier should also be used in structured logging entry and
completion events so command telemetry and machine-readable output line up.

## Canonical Success Envelope

Machine-readable success results should use one stable top-level envelope
family.

Illustrative shape:

```json
{
  "ok": true,
  "command": "lint.sc-boundary",
  "data": {
    "status": "pass",
    "findings": []
  },
  "diagnostics": []
}
```

Required properties:

- top-level success is explicit
- command identity is stable enough for automation and test fixtures
- backend payload lives under a stable field rather than changing the top-level
  JSON shape per backend
- diagnostics are additive and do not replace the business payload

The exact field names may still be tuned, but the envelope family must remain
stable once implemented.

## Canonical Error Envelope

Machine-readable failures should use `CliError` inside the same top-level
contract family.

Illustrative shape:

```json
{
  "ok": false,
  "command": "lint.sc-boundary",
  "error": {
    "kind": "backend_protocol",
    "code": "CLI.BACKEND_PROTOCOL_ERROR",
    "message": "sc-lint-boundary returned unexpected JSON",
    "details": {
      "tool": "sc-lint-boundary"
    },
    "suggested_action": "Re-run with the matching sc-lint workspace revision"
  },
  "diagnostics": []
}
```

`CliError` structure:

- `kind`
- `code`
- `message`
- `details`
- `suggested_action`

`details` and `suggested_action` may be omitted when they do not apply, but
the machine-readable failure family must remain stable.

## Error Kinds

The initial documented top-level error categories should include:

- `usage`
- `config`
- `capability`
- `backend_failure`
- `backend_protocol`
- `internal`

These are CLI-level categories. Backends may carry more specific rule or
domain codes beneath them.

### Error kind to stable code mapping

The initial documented mapping should be:

| Error kind | Stable code family | Typical meaning |
| --- | --- | --- |
| `usage` | `CLI.USAGE_ERROR` | invalid arguments or unsupported command shape |
| `config` | `CLI.CONFIG_ERROR` | repo config missing, malformed, or contradictory |
| `capability` | `CLI.CAPABILITY_ERROR` | optional capability such as `cargo xwin` is required but unavailable |
| `backend_failure` | `CLI.BACKEND_EXEC_FAILURE` | delegated backend failed to execute cleanly or returned a typed failure |
| `backend_protocol` | `CLI.BACKEND_PROTOCOL_ERROR` | delegated backend returned malformed or unexpected machine output |
| `internal` | `CLI.INTERNAL_ERROR` | top-level CLI bug or invariant violation |

The exact string values may still be tuned before implementation, but the
release-1 contract must define one stable mapping from top-level error kind to
top-level stable code.

## Planned Command-Family Contract Matrix

Every non-interactive command family should be implementation-reviewed against
the same matrix before code lands:

| Command family | Stable `command` pattern | Success payload owner | Applicable top-level error kinds |
| --- | --- | --- | --- |
| `lint` | `lint.<tool-or-profile>` | analyzer backend or lint-profile orchestrator | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` |
| `view` | `view.<target>` | view/report backend or adapter layer | `usage`, `config`, `backend_failure`, `backend_protocol`, `internal` |
| `check` | `check.<target>` | compile/preflight runner | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` |
| `clippy` | `clippy.<target>` | lint-runner backend | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` |
| `ci` | `ci` | top-level orchestration layer | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` |
| `version` | `version` | top-level CLI crate | `usage`, `internal` |

This matrix exists to prevent each command family from inventing its own
response or error pattern at implementation time.

## Backend-to-CLI Normalization

The top-level CLI must normalize backend-native results into the canonical
contract.

### Rust library backend

When the CLI calls a Rust library directly:

- backend success payloads become `CommandEnvelope<T>.data`
- typed backend errors are mapped into `CliError`
- backend-specific details may be retained under `details`
- the top-level CLI remains responsible for the final `kind` / `code`
  normalization

### Rust binary backend

When the CLI dispatches to a specialized binary:

- the binary must be invoked in machine mode
- the CLI must parse the backend machine payload
- the backend payload is then normalized into the top-level envelope

If the delegated binary:

- exits nonzero with a valid machine-readable failure payload
  - map that payload into `CliError`
- exits nonzero without a valid machine-readable payload
  - emit `CLI.BACKEND_EXEC_FAILURE`
- exits zero with malformed machine-readable output
  - emit `CLI.BACKEND_PROTOCOL_ERROR`

### Python backend

When the CLI dispatches to a Python utility:

- the Python tool must be invoked through a stable machine-output path
- its success/failure payloads must be normalized into the top-level envelope
- Python traceback text must not become the public machine contract

If a Python utility does not yet expose an adequate machine-readable path, the
CLI must use an adapter layer and treat the adapter output as the contract
boundary rather than leaking raw Python stderr.

## Exit-Code Mapping

Exit codes remain top-level CLI concerns.

Recommended initial policy:

- `0`
  - command succeeded
- nonzero
  - command failed

The CLI may use finer-grained exit-code families later, but those codes must be
documented alongside the machine-readable contract and must not drift per
backend.

## Relationship To Backend JSON

Backends may already expose machine-readable contracts of their own, such as:

- `sc-lint-boundary analyze --format json`

Those backend contracts remain important, but they are not the final
user-facing `sc-lint` contract.

The top-level CLI should preserve backend business payloads while still
normalizing:

- envelope shape
- failure shape
- exit-code behavior

## Human Output

Human-readable output must be a rendering of the same underlying command
result.

It must not:

- contain machine-significant information missing from `--json`
- silently hide failure categories that exist in `CliError`
- become the only supported path for debugging backend dispatch failures

## Consistency Gates

Implementation is not considered complete unless tests prove that:

- every non-interactive command family uses the same top-level envelope keys
- every failure path uses `CliError` rather than family-specific JSON
- `command` values match the documented dotted-identifier convention
- delegated backends cannot bypass the top-level normalization path

## Graph and Interactive Futures

Future graph exploration or TUI features may add richer human workflows, but
they must not replace the documented machine contract.

Any graph data that matters to automation must remain available through the
same top-level machine-readable surface.
