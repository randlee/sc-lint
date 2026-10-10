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
- `sc-lint lint --consumer --config sc-lint.toml [SELECTOR] [SCOPE]`
  (also `lint --consumer ci`, `lint ci --consumer`,
  `lint --consumer ci --profile <selector>`)
  - `lint.ci.consumer`
- `sc-lint test --config sc-lint.toml [SELECTOR...] [-- ARGS...]`,
  `sc-lint test --list`
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
  - `check.interfaces`
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
- `sc-lint provision`
  - `provision`

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
  - implemented consumer lint selector run (REQ-CLI-023); `data` per
    *Consumer Step Payload*
- `test`
  - implemented consumer test selector run and `--list` (REQ-CLI-023); `data`
    per *Consumer Step Payload*
- `init`
  - implemented product-owned consumer integration renderer with check,
    dry-run, idempotency, and conflict behavior
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

Delegated analyzer reports (`lint.sc-portability`, `lint.sc-runtime`) carry the
backend `FindingsReport` unchanged in `data`. An analyzer whose
`[tool.sc-lint.analyzers.<name>]` table sets `enabled = false` reports:

    {"findings": [], "scanned_crates": 0, "schema_version": "0.1.0",
     "status": "pass", "tool": "sc-lint-runtime", "version": "<version>"}

A backend configuration error (unknown key, bad selector, bad pattern) exits
the backend with code 2 and a single `error: <message>` stderr line; `sc-lint`
reports it as an error envelope whose cause is that line, with kind `usage`
(exit 2) when the line names `--rule-filter` and `config` (exit 3) otherwise
(see *Rust binary backend*).

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

Business-verdict rule (REQ-CLI-005J):

- `sc-lint lint sc-boundary`, `sc-lint lint sc-portability`, and
  `sc-lint lint sc-runtime` are business-verdict commands: a completed analysis
  is always a `CommandEnvelope<T>` success (`ok: true`) with the backend report
  under `data`
- the consumer selector runs `sc-lint lint --consumer` (`lint.ci.consumer`)
  and `sc-lint test` (`test`) are business-verdict commands: a run whose steps
  all started is a success envelope whose `data.status` is `"fail"` when any
  step exited nonzero; `sc-lint test --list` reports `"pass"`
- when `data.status` is `"fail"` the process exits `7` (`verdict_fail`); any
  other status exits `0`; stdout carries the same envelope in both cases, no
  error envelope is written, and the analyzer commands write nothing to stderr
- `ok: true` with exit `7` is the verdict shape; it is never a `CliError`
  reclassification, and `CliError` exit codes `1`-`6` keep their meaning
- the human rendering of the analyzer commands is
  `<tool>: <status> (<n> findings)`
- the built-in profiles (`lint fast|full|ci`, `ci`) are not business-verdict
  commands; a member step that exits `7` fails the profile with a `CliError`
  (exit `5`)
- `sc-lint check interfaces` remains a planned business-verdict command: a
  completed comparison that detects breaking changes stays a
  `CommandEnvelope<T>` result with the negative verdict carried inside `data`

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
| `compatibility check` | `compatibility.check` | compatibility preflight | `config`, `backend_failure`, `internal` |

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
- selector commands are argv arrays, not shell strings. Each runs in the
  configuration directory or its `cwd`, with `SC_LINT_BIN` (the running
  sc-lint binary) and then its `env` overlaid; its stdout is copied to
  sc-lint's stderr and its stderr is inherited, so stdout carries only the
  envelope. Steps of a parallel batch are the exception: their stdout and
  stderr are captured together and written to sc-lint's stderr between
  ``sc-lint: begin output of step `<name>` `` and ``sc-lint: end output of
  step `<name>`: <result>`` lines, one block per step in declaration order.
  A step that exits nonzero is reported in `data.steps` with
  `status: "fail"`; the envelope stays `ok: true` and the command exits `7`
  (`verdict_fail`). A step that cannot start is a structured `CliError`; in
  a batch it is returned after the other batch steps finish, for the
  lowest-index step that could not start.
- missing configured executables use
  `CLI.SC_LINT_BACKEND_NOT_FOUND` with installation recovery and the setup
  documentation reference.
- a step whose program is exactly `sc-lint` runs the current sc-lint binary.
  A step whose program is exactly `python` runs the interpreter of
  `[tool.sc-lint.runner] python` when that table is set; a missing
  interpreter uses `CLI.SC_LINT_RUNNER_PYTHON_MISSING`. A running step whose
  `requires` tool is not installed uses `CLI.SC_LINT_TOOL_NOT_PROVISIONED`. A
  running step that needs `cargo xwin` while it is unavailable uses
  `CLI.CAPABILITY_ERROR`, except under `all` for a step whose `platforms`
  exclude the host, which is reported `skipped`. All three are `capability`
  errors (exit `4`) raised before any step runs. `data.steps[].command` is
  the argv actually run, including `xwin` and `--target` added by `target`.

The generated default configuration uses this shape:

```toml
[[tool.sc-lint.lint]]
name = "fmt"
command = ["cargo", "fmt", "--all", "--check"]

[[tool.sc-lint.test]]
name = "workspace"
command = ["cargo", "test", "--workspace"]
```

The 0.5 `[[tool.sc-lint.lint]]` / `[[tool.sc-lint.test]]` array form above
remains valid. Since 0.6.1 a configuration may instead use the table form,
which names selectors and groups (REQ-CLI-023):

```toml
[tool.sc-lint.lint]
default = ["fast", "clippy"]

[tool.sc-lint.lint.selectors.fmt]
command = ["cargo", "fmt", "--all", "--check"]

[tool.sc-lint.lint.selectors.clippy]
command = ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]

[tool.sc-lint.lint.selectors.clippy-windows]
command = ["cargo", "xwin", "clippy", "--target", "x86_64-pc-windows-msvc", "--workspace", "--", "-D", "warnings"]
platforms = ["windows"]

[tool.sc-lint.lint.selectors.fast]
group = ["fmt"]

[tool.sc-lint.lint.selectors.ci]
group = ["fast", "clippy", "clippy-windows"]

[tool.sc-lint.test]
default = ["unit"]

[tool.sc-lint.test.selectors.unit]
command = ["cargo", "test", "--workspace"]

[tool.sc-lint.test.selectors.e2e]
command = ["cargo", "test", "--test", "e2e"]
platforms = ["linux", "macos"]

[tool.sc-lint.test.selectors.full]
group = ["unit", "e2e"]
```

The table form also accepts `jobs`, `parallel`, `target` and `requires`, and
the top-level `tools`, `setup` and `runner` tables (REQ-CLI-015,
REQ-CLI-019, REQ-CLI-023):

```toml
[tool.sc-lint.lint]
default = ["ci"]
jobs = 4

[tool.sc-lint.lint.selectors.version]
command = ["python", "tools/check_version.py"]
parallel = true

[tool.sc-lint.lint.selectors.lines]
command = ["python", "tools/check_lines.py"]
parallel = true

[tool.sc-lint.lint.selectors.clippy-windows]
command = ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]
platforms = ["windows"]
target = "x86_64-pc-windows-msvc"
requires = ["cargo-xwin", "x86_64-pc-windows-msvc"]

[tool.sc-lint.lint.selectors.ci]
group = ["version", "lines", "clippy-windows"]

[tool.sc-lint.tools]
rustup_targets = ["x86_64-pc-windows-msvc"]

[tool.sc-lint.tools.cargo]
cargo-xwin = "0.18.4"

[tool.sc-lint.setup]
pre = ["python3", "tools/setup.py"]

[tool.sc-lint.runner]
python = ".venv"
```

`version` and `lines` form one parallel batch. `tools.cargo` maps a crate
name to an exact SemVer version, `rustup_targets` lists target triples, and
`requires` names entries of either. `runner.python` is a virtual-environment
directory relative to the configuration directory. `jobs` affects timing
only, never the report.

### Consumer Step Payload

`sc-lint --json lint --consumer ci all` on a Linux host without `cargo xwin`,
with `keep_going = true`, exits `7`:

```json
{"ok":true,"command":"lint.ci.consumer","data":{
 "status":"fail","profile":"lint","form":"table","selector":"ci","selectors":["ci"],
 "scope":"all","host_platform":"linux","keep_going":true,"args":[],
 "step_count":5,"counts":{"pass":3,"fail":1,"skipped":1,"not_run":0},
 "summary":"3 passed, 1 failed, 1 skipped, 0 not run",
 "steps":[
  {"name":"fmt","kind":"lint","command":"cargo fmt --all --check","status":"pass"},
  {"name":"version","kind":"lint","command":"python tools/check_version.py","status":"pass"},
  {"name":"clippy","kind":"lint","command":"cargo clippy --workspace --all-targets -- -D warnings","status":"fail","exit_code":101},
  {"name":"clippy-windows","kind":"lint","command":"cargo xwin clippy --target x86_64-pc-windows-msvc --workspace -- -D warnings","status":"skipped","reason":"requires `cargo xwin`, which is not available on host `linux`"},
  {"name":"lines","kind":"lint","command":"python tools/check_lines.py","status":"pass"}]},
 "diagnostics":[]}
```

`data` fields:

- `status`: `"fail"` when any step has status `"fail"`, otherwise `"pass"`
- `profile`: `"lint"` or `"test"`
- `form`: `"legacy"` (array form) or `"table"`
- `selector`: `"default"`, `"ci"`, `"all"`, or the requested names joined
  with a space
- `selectors`: the resolved selector names before group expansion
- `scope`: `host`, `all`, `linux`, `macos` or `windows`; always `host` for
  `test`
- `host_platform`: `linux`, `macos`, `windows`, or `null` on another OS
- `keep_going`: the table value (`false` for the legacy form)
- `args`: the arguments given after `--` (`test` only, otherwise `[]`)
- `step_count`: the number of expanded steps
- `counts`: `pass`, `fail`, `skipped` and `not_run` step counts
- `summary`: `"<pass> passed, <fail> failed, <skipped> skipped, <not_run> not run"`
- `steps[]`: `name`, `kind` (`lint` or `test`), `command` (argv joined with
  spaces), `status` (`pass`, `fail`, `skipped`, `not_run`), `exit_code`
  (`fail` with an exit code only), `reason` (`skipped` and `not_run` only),
  `parallel` (`true` only for a step that ran in a parallel batch; otherwise
  absent)

Step reasons (`{h}` is the host platform or `unknown`, `{p}` the requested
platform, `{list}` the step's `platforms` joined with `, `, `{name}` the
failing step):

```text
host platform `{h}` is not in platforms [{list}]
platform scope `{p}` is not in platforms [{list}]
requires `cargo xwin`, which is not available on host `{h}`
stopped after failing step `{name}` (keep_going = false)
```

The first is used under scope `host`, the second under an explicit
`<platform>`, the third under `all` for a step whose `platforms` exclude the
host and that would run with `cargo xwin`, and the fourth for steps after a
failure when `keep_going = false`. Any other running step that would run with
`cargo xwin` while it is unavailable is a capability error (exit `4`) before
any step runs (REQ-CLI-015).

`sc-lint --json test --list` exits `0`:

```json
{"ok":true,"command":"test","data":{
 "status":"pass","profile":"test","mode":"list","form":"table","default":["unit","python"],
 "summary":"5 selectors",
 "selectors":[
  {"name":"coverage","kind":"command","default":false,"description":null,"command":["cargo","llvm-cov","--workspace"],"platforms":[],"cwd":null,"env":{},"parallel":false,"target":null,"requires":[]},
  {"name":"e2e","kind":"command","default":false,"description":null,"command":["cargo","test","--test","e2e"],"platforms":["linux","macos"],"cwd":null,"env":{},"parallel":false,"target":null,"requires":[]},
  {"name":"full","kind":"group","default":false,"description":null,"members":["unit","python","e2e"]},
  {"name":"python","kind":"command","default":true,"description":null,"command":["python","-m","unittest","discover","-s","tests"],"platforms":[],"cwd":null,"env":{},"parallel":false,"target":null,"requires":[]},
  {"name":"unit","kind":"command","default":true,"description":"Rust unit tests","command":["cargo","test","--workspace"],"platforms":[],"cwd":null,"env":{},"parallel":false,"target":null,"requires":[]}]},
 "diagnostics":[]}
```

Selectors are listed in lexicographic order for the table form and in
declaration order for the legacy form. Command selectors report `parallel`,
`target` (string or `null`) and `requires` (array) after `env`.

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
and the `sc-lint-docs` bundle sit beside the running executable, as they do in
a released archive; `false` for source builds under `target/`. The `full` and
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

`sc-lint setup [--check] [--dry-run]` and `sc-lint upgrade [--check]
[--dry-run]` load the same `[tool.sc-lint].minimum_version` requirement as
compatibility check.
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

`setup --check` and `upgrade --check` never change disk and report `current` or
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
| `CLI.SC_LINT_TOOL_INSTALL_FAILED` | Fix the reported tool error and rerun `just setup`. |

Every installer failure uses `backend_failure`, a stable code, a cause,
recovery guidance, and `docs: "sc-lint docs installation"`. The E.4 bundle
will provide that offline guide; E.2 does not implement documentation delivery.

`sc-lint provision [--check] [--dry-run]` loads the consumer configuration,
runs `[tool.sc-lint.setup] pre` in the configuration directory (install mode
only), then installs each missing `[tool.sc-lint.tools]` entry (`rustup
target add <triple>`, then `cargo install --locked <name> --version
<version>`) and re-verifies it. It reports `status` (`provisioned`,
`current`, `update_required`, `dry_run`), `mode`, `config_path`, `pre`
(`command`, `status`, `reason`, or null), `tools[]` (`kind`, `name`,
`version`, `installed_version`, `status`: `present`|`installed`|`missing`),
`counts` and `summary`. `--check` and `--dry-run` change nothing. A failing
`setup.pre` is `backend_failure` (`CLI.BACKEND_EXEC_FAILURE`, message
`setup.pre failed`); a failed install or re-verification is
`backend_failure` with `CLI.SC_LINT_TOOL_INSTALL_FAILED`. The bootstrap's
`setup` operation runs `provision` after the product `setup` (REQ-CLI-021).

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
- `sc-lint lint sc-boundary` passes `--rule-filter` and the resolved config
  path into `AnalyzeOptions`; a `--rule-filter` selector error maps to
  `usage` (exit 2), any other `[tool.sc-lint.analyzers.boundary]` error maps
  to `config` (exit 3), and a disabled boundary table reports `status:
  "pass"`, `scanned_crates: 0`, `findings: []` with `tool:
  "sc-lint-boundary"`

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
- exits zero with a valid report whose `status` is `"fail"`
  - emit the success envelope and exit `7` (`verdict_fail`); see the
    business-verdict rule

`sc-lint lint <analyzer>` invokes `analyze --root <root> --format json`, then
appends `--config <path>` when a config file was given or found, then
`--rule-filter <value>` when given. A delegated backend that exits 2 maps to
`usage` (exit 2) when its stderr names `--rule-filter`, otherwise to
`config` (exit 3); the stderr text is the error `cause`. Other nonzero exits
keep `CLI.BACKEND_EXEC_FAILURE`.

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

Policy:

- `0`
  - command succeeded (for business-verdict commands: verdict `pass`)
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
- `7`
  - `verdict_fail`: a business-verdict command (`lint sc-boundary`,
    `lint sc-portability`, `lint sc-runtime`, or a consumer selector run with
    a failing step) completed and its payload verdict is negative; the
    envelope is a success (`ok: true`) and no `CliError` exists
    (REQ-CLI-005J)

These codes are owned by the CLI and must not drift per backend. `1`-`6` map
one-to-one to `CliError.kind`; `7` is never produced by a `CliError`.

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
- business-verdict exit `7` for `lint.sc-boundary`, `lint.sc-portability`,
  and `lint.sc-runtime` with an `ok: true` envelope, and exit `0` on `pass`
  (`crates/sc-lint/src/tests.rs`, `crates/sc-lint/tests/verdict_exit.rs`)

## Graph and Interactive Futures

Future graph exploration or TUI features may add richer human workflows, but
they must not replace the documented machine contract.

Any graph data that matters to automation must remain available through the
same top-level machine-readable surface.
