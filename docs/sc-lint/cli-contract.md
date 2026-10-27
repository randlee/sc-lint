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
- `sc-lint lint function-length`
  - `lint.function-length`
- `sc-lint lint line-counts`
  - `lint.line-counts`
- `sc-lint lint identity-literals`
  - `lint.identity-literals`
- `sc-lint lint fast`
  - `lint.fast`
- `sc-lint lint --consumer --config sc-lint.toml ci`
  - `lint.ci.consumer`
- `sc-lint test [--fail-fast|--list] [<selector> [args...]]`
  - `test`
- `sc-lint init --just`
  - `init`
- `sc-lint view findings`
  - `view.findings`
- `sc-lint view graph`
  - `view.graph`
- `sc-lint check xwin`
  - `check.xwin`
- `sc-lint check interfaces`
  - `check.interfaces` (planned, not implemented)
- `sc-lint clippy xwin`
  - `clippy.xwin`
- `sc-lint ci`
  - `ci`
- `sc-lint version`
  - `version`
- `sc-lint --version`
  - `version`
- `sc-lint compatibility check`
  - `compatibility.check`
- `sc-lint setup`
  - `setup`
- `sc-lint upgrade`
  - `upgrade`

When parsing fails before a concrete command path is resolved, the CLI uses the
fallback identifier:

- parser-level usage failure
  - `cli.parse_error`

The same identifier should also be used in structured logging entry and
completion events so command telemetry and machine-readable output line up.

Current implementation status:

- `version`
  - direct CLI-owned success path
- `compatibility.check`
  - implemented consumer installation preflight; it uses only the canonical
    `sc-lint.toml` requirement and the installed binary's version probe
- `lint.sc-boundary`
  - first real backend-normalized success path
- `lint.fast`
  - implemented profile orchestration path
- `lint.full`
  - implemented profile orchestration path with conditional `xwin` preflight
- `lint.ci`
  - implemented lint-only CI-parity profile path
- `lint.ci.consumer`
  - implemented explicit consumer lint profile path; its configured argv steps
    run from `sc-lint.toml` without source-checkout discovery
- `test`
  - implemented explicit consumer test profile path: `sc-lint test
    [--fail-fast|--list] [<selector> [args...]]` runs the default selector (or
    every `[[tool.sc-lint.test]]` row), the named selector with forwarded
    arguments, or lists the declared selectors
- `init`
  - implemented product-owned consumer integration renderer with check,
    dry-run, idempotency, and conflict behavior
- `check.native`
  - implemented native preflight path
- `check.xwin`
  - implemented capability-gated Windows preflight path
- `check.interfaces`
  - planned, not implemented Phase `C` interface-versioning command path
- `clippy.native`
  - implemented native clippy path
- `clippy.xwin`
  - implemented capability-gated Windows clippy path
- `ci`
  - implemented top-level lint-plus-tests path
- `lint.function-length`
  - implemented Python-adapter lint path
  - not in the `Ci` profile (`lint.ci`), like `lint.line-counts` and `lint.identity-literals`; gated by `just lint` and the `Full` profile
  - a stale baseline allowance (its function shrank or went away) fails the gate until `--write-baseline` ratchets it down, so a recorded allowance never permits regrowth
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

Business-verdict behavior:

- planned, not implemented: `sc-lint check interfaces` is a business-verdict command rather than a
  transport or protocol failure surface
- `sc-lint check interfaces` is planned, not implemented; `CheckTarget` offers only `native` and `xwin`, so the CLI rejects it as an unknown target (`cli.parse_error`, `usage`, exit `2`). Once implemented it is intended to be a business-verdict command (REQ-CLI-005J).
- `lint sc-boundary`, `lint sc-portability` and `lint sc-runtime` now exit `10`
  when `data.status = "fail"`; the envelope remains `ok: true`

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
- `docs`

`cause`, `details`, `suggested_action`, and `docs` may be omitted when they do not
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

## Stable codes

| Code | Kind | Cause names | Recovery |
| --- | --- | --- | --- | --- |
| `config.boundaries_dir_missing` | `config` | the repo root searched | create `boundaries/` or do not run the boundary family |
| `config.table_missing` | `config` | the table and files searched | add the table to `sc-lint.toml` or `.just/lint-config.toml` |
| `profile.invalid_after_all` | `config` | the step or group selector and file | remove after_all from that selector |
| `test.unknown_selector` | `config` | the selector name and file | use a name from sc-lint test --list |
| `test.group_cycle` | `config` | the selector cycle path | break the cycle in [tool.sc-lint.test] |
| `profile.steps_failed` | `backend_failure` | failed_steps[]: index, name, exit_code, message in declaration order (skipped_steps[] under --fail-fast) | fix the failing steps and rerun |
| `backend_payload_drift` | `backend_protocol` | the failing JSON key path | upgrade or reinstall the wheel so the payload matches the envelope schema |

Codes are typed constants in their owning module (`ConfigCode` in `config.rs`; `CONFIG_TABLE_MISSING` in `lint_common.py`; adapter payload drift in `python_adapter.rs`) and cause text interpolates names.

## Command-Family Contract Matrix

Every non-interactive command family must be implementation-reviewed against
the same matrix before code lands:

| Command family | Stable `command` pattern | Success payload owner | Applicable top-level error kinds | Enforcing test |
| --- | --- | --- | --- |
| `lint` | `lint.<tool-or-profile>` | analyzer backend or lint-profile orchestrator | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` | `lint_sc_boundary_normalizes_backend_success_through_the_top_level_envelope`, `python_backed_lints_and_views_normalize_through_the_top_level_envelope` |
| `view` | `view.<target>` | view/report backend or adapter layer | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` | `reserved_view_commands_use_the_same_failure_envelope_shape`, `envelope_key_set_identical_across_families` |
| `check` | `check.<target>` | compile/preflight runner | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` | `explicit_xwin_commands_require_capability`, `envelope_key_set_identical_across_families` |
| `clippy` | `clippy.<target>` | lint-runner backend | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` | `native_and_xwin_preflight_commands_use_success_envelopes`, `envelope_key_set_identical_across_families` |
| `ci` | `ci` | top-level orchestration layer | `usage`, `config`, `capability`, `backend_failure`, `backend_protocol`, `internal` | `envelope_key_set_identical_across_families` |
| `version` | `version` | top-level CLI crate | `usage`, `internal` | `version_success_uses_the_canonical_top_level_envelope` |
| `compatibility check` | `compatibility.check` | compatibility preflight | `config`, `backend_failure`, `internal` | `envelope_key_set_identical_across_families` |
| `docs` | `docs` | offline documentation discovery (`docs.rs`) | `usage`, `capability` | `envelope_key_set_identical_across_families` |
| `init` | `init` | consumer integration generator | `usage`, `config`, `internal` | `envelope_key_set_identical_across_families` |
| `test` | `test` | consumer test-profile orchestration | `usage`, `config`, `backend_failure`, `internal` | `envelope_key_set_identical_across_families` |
| `setup` | `setup` | managed installer | `usage`, `config`, `backend_failure`, `internal` | `setup_and_upgrade_json_use_the_canonical_top_level_envelope` |
| `upgrade` | `upgrade` | managed installer | `usage`, `config`, `backend_failure`, `internal` | `setup_and_upgrade_json_use_the_canonical_top_level_envelope` |
| `lint --consumer` | `lint.ci.consumer` | consumer lint-profile orchestration | `usage`, `config`, `capability`, `backend_failure`, `internal` | `envelope_key_set_identical_across_families` |
| parser failure | `cli.parse_error` (fixed fallback) | clap parse layer, `entry.rs` `handle_parse_error` | `usage` | `parse_errors_use_the_documented_command_identifier`, `envelope_key_set_identical_across_families` |

This matrix exists to prevent each command family from inventing its own
response or error pattern at implementation time.

## Compatibility And Version-Probe Contract

## Consumer Integration And Profile Contract

Consumer mode is selected by command shape, never by a repository name,
Cargo manifest, or analyzer package:

- `sc-lint init --just` writes the generated config, Justfile, and bootstrap
  helper only when each path is missing or product-managed. Its CLI `--json`
  result uses the standard command envelope; the generated POSIX bootstrap is
  intentionally a plain-text, exit-code-oriented preflight helper and is not a
  JSON-envelope producer.
- `sc-lint lint --consumer --config sc-lint.toml ci` and `sc-lint test
  --config sc-lint.toml` load the configuration beside the consumer project.
- profile commands are argv arrays, not shell strings. Each command runs in
  the configuration directory (or a test selector's `cwd` below it). Steps
  run in parallel (cargo `argv[0]` steps serialized in declaration order,
  `after_all` steps last) and fail slow: every step runs, and any failure
  fails the aggregate command with one `profile.steps_failed` error.
- `profile.steps_failed` (exit 5) carries `details.failed_steps[]`, one
  `{"index", "name", "exit_code", "message"}` per failed step in declaration
  order; `index` is the zero-based position in `data.steps[]` (platform-skipped
  rows included), `exit_code` is `null` when the process did not start or the
  step panicked, and `message` is the step error and its cause. Under
  `--fail-fast` it also carries `details.skipped_steps[]`, one
  `{"index", "name", "reason": "fail-fast"}` per step that never started.
- a missing configured executable is a failed step whose `message` starts
  with `<name> failed to start` and includes the spawn error.
- `sc-lint test --list` runs nothing and returns
  `{"status": "pass", "profile": "test", "default": <name|null>, "selectors":
  [{"name", "kind": "command"|"group", "command": [...] | "group": [...],
  "platforms": [...], "cwd": <string|null>, "env_keys": [...], "after_all":
  bool}]}`, sorted by name.

The generated default configuration uses this shape:

```toml
[[tool.sc-lint.lint]]
name = "fmt"
command = ["cargo", "fmt", "--all", "--check"]

[[tool.sc-lint.test]]
name = "workspace"
command = ["cargo", "test", "--workspace"]
```

The only consumer minimum-version location is:

```toml
[tool.sc-lint]
minimum_version = "0.4.1"
```

`sc-lint --json version` is intentionally independent of a source checkout and
emits this payload under the standard success envelope's `data` field:

```json
{
  "tool": "sc-lint",
  "version": "0.4.1",
  "contract_schema": "sc-lint-version-v1",
  "self_contained": true,
  "status": "pass"
}
```

`self_contained` reports the archive layout: `true` when every declared
backend binary (`sc-lint-boundary`, `sc-lint-portability`, `sc-lint-runtime`)
and the `sc-lint-docs` bundle resolve from a supported installed layout,
including the published archive's `bin/` and manifest-declared bundle paths;
`false` for source builds under `target/`. The `full` and
`ci` lint profiles re-enter the running executable for their `sc-boundary` and
`sc-portability` steps, so a released archive runs every kit recipe with no
source-tree helper (issue #84).

`sc-lint compatibility check` loads the requirement once and runs
`sc-lint --json version` against the PATH installation (or `--binary <path>`).
It performs no lint/test work and creates no logs or reports. Its success data identifies
`minimum_version`, `installed_version`, `binary_path`, and `config_path`.

The preflight failure codes are stable:

| Code | Recovery condition |
| --- | --- |
| `CLI.SC_LINT_CONFIG_MISSING` | Create or select canonical `sc-lint.toml`. |
| `CLI.SC_LINT_CONFIG_MALFORMED` | Repair `[tool.sc-lint].minimum_version`. |
| `CLI.SC_LINT_BINARY_NOT_FOUND` | Run `just setup` or the product installer. |
| `CLI.SC_LINT_BINARY_EXECUTION_FAILED` | Repair the selected installation, then rerun setup. |
| `CLI.SC_LINT_VERSION_PROBE_MALFORMED` | Install a release implementing `sc-lint-version-v1`. |
| `CLI.SC_LINT_VERSION_UNPARSABLE` | Install a release implementing `sc-lint-version-v1`. |
| `CLI.SC_LINT_VERSION_TOO_OLD` | Run `just setup` to install or upgrade the required version. |

Every one of these errors includes `cause`, `suggested_action`, and
`docs: "sc-lint docs setup"`; its details include the required version and
available observed version/path.

Compatibility preflight uses `config` only for the repository requirement.
Failures while locating, executing, or validating the external installed binary
use `backend_failure`, not `capability`: that category records a concrete
subprocess/install failure rather than the absence of an optional feature.

## Installation And Upgrade Contract

`sc-lint setup [--dry-run]` and `sc-lint upgrade [--check] [--dry-run]` load
the same `[tool.sc-lint].minimum_version` requirement as compatibility check.
That field remains an installed-version floor; the installer reports the exact
`selected_release_version` separately for the immutable release artifact it
downloads. They select the release workflow's host archive
`sc-lint_<version>_<target>.<tar.gz|zip>` and its sibling `checksums.txt` from
the `v<version>` release. The installer stages download and extraction, checks
SHA-256 before activation, and verifies `sc-lint --json version` after an
atomic activation. It rolls back the prior managed binary when activation or
post-install verification fails. If rollback cannot be verified, the command
does not claim recovery succeeded: it returns the backup path and manual
recovery instructions instead.

`upgrade --check` never changes disk and reports `current` or
`update_required`; `--dry-run` reports the archive, checksum manifest, and
managed target that would be used. A compatible newer binary is never
downgraded. The managed directory is platform-local by default and may be set
explicitly with `SC_LINT_INSTALL_DIR`; `SC_LINT_RELEASE_BASE_URL` is a release
fixture/enterprise mirror override.

| Code | Recovery condition |
| --- | --- |
| `CLI.SC_LINT_INSTALL_UNSUPPORTED_PLATFORM` | Use a published target or install manually. |
| `CLI.SC_LINT_RELEASE_UNAVAILABLE` | Restore network/release access and retry setup. |
| `CLI.SC_LINT_RELEASE_CHECKSUM_MISMATCH` | Do not activate the artifact; redownload from the official release. |
| `CLI.SC_LINT_INSTALL_PERMISSION_DENIED` | Choose a writable managed install directory. |
| `CLI.SC_LINT_POST_INSTALL_VERSION_FAILED` | A prior managed binary, if one existed, was restored; otherwise the failed candidate was removed. Repair the release and rerun setup. |
| `CLI.SC_LINT_INSTALL_ROLLBACK_FAILED` | Inspect the reported target and backup path; restore the known-good backup manually before retrying. |
| `CLI.SC_LINT_INSTALL_ACTIVATION_FAILED` | The previous managed binary was retained; repair the target directory and retry. |

Every installer failure uses `backend_failure`, a stable code, a cause,
recovery guidance, and `docs: "sc-lint docs installation"`. The E.4 bundle
will provide that offline guide; E.2 does not implement documentation delivery.

On Windows, an executable cannot reliably replace itself while still running.
When the managed binary is the active `sc-lint.exe`, setup fails before moving
any files and directs the operator to run setup from a separately downloaded
executable. The normal activation sequence remains rename target to retained
backup, replace from the verified staging directory, then post-install probe;
CI exercises the Windows checksum path independently.

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
- `sc-lint lint sc-boundary|sc-portability|sc-runtime` forwards `--config` and
  `--rule-filter` unchanged; a backend exit 2 becomes a usage error and exit 3
  a config error.

### Python adapter backend

For Python-backed utility paths in A.3:

- the CLI invokes the Python tool with `--json`
- the Python tool emits `sc-lint-python-v1`, whose documented payload shape is validated before normalization: `adapter_schema` is a string, `ok` is a boolean, and `diagnostics` is an array of strings; success requires object `data` and string `summary` with no `error`; failure requires string `error.kind` and `error.message`, permits object `error.details`, string `error.suggested_action` and string `summary`, and has no `data`; unknown additive keys are allowed
- a payload that matches the adapter version but drifts from this shape fails as `backend_protocol` with code `backend_payload_drift`, including when the Python process exits non-zero
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

Current CLI exit-code mapping:

- `0`
  - command completed successfully; analyzer lint commands use this for a
    passing verdict, while a failed analyzer verdict uses code `10`
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
- `10`
  - `lint sc-boundary`, `lint sc-portability` or `lint sc-runtime` completed
    with `data.status = "fail"` (business verdict; the envelope stays
    `ok: true`)

During `sc-boundary` analysis, failure to load the boundary inventory or the
Cargo workspace metadata required to validate it is a configuration failure:
the CLI returns `CLI.CONFIG_ERROR` with exit code `3`. A later failure while
building the workspace graph remains a delegated backend execution failure
(`CLI.BACKEND_EXEC_FAILURE`, exit code `5`).

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
