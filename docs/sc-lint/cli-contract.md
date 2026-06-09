# sc-lint CLI Contract

This document defines the end-to-end contract for the top-level
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

## Contract Types

The release-1 contract explicitly names these types:

- `Cli`
- `Command`
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
- `sc-lint lint line-counts`
  - `lint.line-counts`
- `sc-lint lint identity-literals`
  - `lint.identity-literals`
- `sc-lint lint fast`
  - `lint.fast`
- `sc-lint view findings`
  - `view.findings`
- `sc-lint view graph`
  - `view.graph`
- `sc-lint check xwin`
  - `check.xwin`
- `sc-lint check interfaces`
  - `check.interfaces`
- `sc-lint clippy xwin`
  - `clippy.xwin`
- `sc-lint ci`
  - `ci`
- `sc-lint version`
  - `version`
- `sc-lint --version`
  - `version`

When parsing fails before a concrete command path is resolved, the CLI uses the
fallback identifier:

- parser-level usage failure
  - `cli.parse_error`

The same identifier should also be used in structured logging entry and
completion events so command telemetry and machine-readable output line up.

Current implementation status:

- `version`
  - direct CLI-owned success path
- `lint.sc-boundary`
  - first real backend-normalized success path
- `lint.fast`
  - implemented profile orchestration path
- `lint.full`
  - implemented profile orchestration path with conditional `xwin` preflight
- `lint.ci`
  - implemented lint-only CI-parity profile path
- `check.native`
  - implemented native preflight path
- `check.xwin`
  - implemented capability-gated Windows preflight path
- `check.interfaces`
  - planned Phase `C` interface-versioning command path
- `clippy.native`
  - implemented native clippy path
- `clippy.xwin`
  - implemented capability-gated Windows clippy path
- `ci`
  - implemented top-level lint-plus-tests path
- `lint.line-counts`
  - implemented Python-adapter lint path
- `lint.identity-literals`
  - implemented Python-adapter lint path
- `view.findings`
  - implemented Python-adapter view path
- `view.graph`
  - still reserved pending a stable graph contract
- `cli.parse_error`
  - implemented parser-level usage failure envelope path when command parsing
    fails before a concrete subcommand identity exists
- `lint.sc-portability`
  - implemented delegated backend path
- `lint.sc-runtime`
  - implemented delegated backend path

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

Planned Phase `C` business-verdict rule:

- `sc-lint check interfaces` is a business-verdict command rather than a
  transport or protocol failure surface
- a completed comparison that detects breaking changes remains a
  `CommandEnvelope<T>` result with the negative verdict carried inside `data`
- nonzero exit for that negative verdict is planned policy behavior, not a
  `CliError` reclassification

The implemented field names are stable for the Phase A bootstrap line.

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
    "cause": "expected top-level `findings` array",
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
- `cause`
- `details`
- `suggested_action`

`cause`, `details`, and `suggested_action` may be omitted when they do not
apply, but the machine-readable failure family must remain stable.

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

The string values above are the implemented A.1a code families.

Parser-level usage failures emitted before `CommandContext` can resolve a
family-specific path still use this same error taxonomy, but their machine
envelope `command` value is the fallback identifier `cli.parse_error`.

## Planned Command-Family Contract Matrix

Every non-interactive command family should be implementation-reviewed against
the same matrix before code lands:

| Command family | Stable `command` pattern | Success payload owner | Applicable top-level error kinds |
| --- | --- | --- | --- |
| `lint` | `lint.<tool-or-profile>` | analyzer backend or lint-profile orchestrator | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` |
| `view` | `view.<target>` | view/report backend or adapter layer | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` |
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

When the CLI calls a Rust library directly, as A.1b does for
`sc-lint lint sc-boundary`:

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

### Python adapter backend

For Python-backed utility paths in A.3:

- the CLI invokes the Python tool with `--json`
- the Python tool emits `sc-lint-python-v1`
- the CLI validates the adapter schema before exposing any success payload
- adapter-reported failures map into `CliError` by structured fields:
  - `kind`
  - `message`
  - optional `details`
  - optional `suggested_action`
- the public exit code still comes from the normalized top-level `CliError`
  kind rather than the raw Python subprocess status
- raw traceback text is not part of the public machine contract

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
- `1`
  - top-level internal failure
- `2`
  - top-level usage failure
- `3`
  - top-level config failure
- `4`
  - top-level capability failure
- `5`
  - delegated backend execution failure
- `6`
  - delegated backend protocol failure

These codes are owned by the CLI and must not drift per backend.

For Python-adapter command paths in A.3:

- the adapter's process exit status is not the public contract surface
- when the adapter returns a structured failure payload, the CLI maps its
  normalized `CliError.kind` to the same top-level exit codes above
- a Python adapter payload with `kind=config` therefore exits `3`, and
  `kind=capability` exits `4`, even if the Python subprocess chose a different
  nonzero status
- adapter startup failures still map to `CLI.BACKEND_EXEC_FAILURE` / exit code
  `5`
- missing, malformed, or unknown-schema adapter output still maps to
  `CLI.BACKEND_PROTOCOL_ERROR` / exit code `6`

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

The consistency gate lives in `crates/sc-lint/src/tests.rs`.

A.1a proves:

- grouped command parsing for the initial surface
- help output for the grouped command root
- success-envelope serialization for `version`
- failure-envelope serialization for `lint`, `view`, `check`, `clippy`, and
  `ci`
- stable exit-code mapping for CLI-owned failures
- CLI-owned logging entry/completion/error event emission

A.1b extends that gate with:

- repo-root discovery and malformed-config handling
- `lint.sc-boundary` success normalization through the same envelope family
- backend execution failure mapping
- backend protocol failure mapping
- dispatch-start and dispatch-normalized log events for the real backend path

## Graph and Interactive Futures

Future graph exploration or TUI features may add richer human workflows, but
they must not replace the documented machine contract.

Any graph data that matters to automation must remain available through the
same top-level machine-readable surface.
