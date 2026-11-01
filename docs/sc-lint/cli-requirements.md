# sc-lint CLI Requirements

This document defines the detailed requirements for the planned top-level
`sc-lint` CLI crate.

Related ADRs:
- [`./adr/ADR-005-cli-profiles-and-xwin-preflight.md`](./adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- [`./adr/ADR-006-ai-first-cli-contract.md`](./adr/ADR-006-ai-first-cli-contract.md)
- [`./adr/ADR-008-sc-observability-logging.md`](./adr/ADR-008-sc-observability-logging.md)
- [`./adr/ADR-022-tools-and-consumer-configuration.md`](./adr/ADR-022-tools-and-consumer-configuration.md)

ADR-005 governs release-1 profile and `xwin` policy and supersedes earlier
provisional rollout notes for cross-target preflight membership.

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
  The implemented A.1b config-discovery surfaces are:
  - `sc-lint.toml`
  - `.just/lint-config.toml`
  with `--root <path>` as the explicit repo-root override.
  (Amended phase L, #137/default-config gap, Accepted — Rand, 2026-10-26.)
  Resolution is per configuration table for every lint, including
  Python-backed lints (line-counts, identity-literals, version-sync): an
  explicit `--config`
  file first, then the table from `sc-lint.toml` when it has one, otherwise the
  same table from `.just/lint-config.toml`. A table present in `sc-lint.toml`
  is never shadowed by the fallback file, and a repo whose `sc-lint.toml` lacks
  a table that only `.just/lint-config.toml` holds (as in atm-core's kit-rendered
  `sc-lint.toml`) keeps using the fallback table. Python-backed lints fail with
  `config.table_missing` when their configuration table is found in neither
  file; Rust analyzers keep their defaults when their table is absent. The
  repo root is the nearest ancestor holding `sc-lint.toml` or a workspace
  `Cargo.toml`; a `boundaries/` directory is required only by the boundary
  family, and its absence is a structured error for that family alone.
  The Python entry `sc_lint.check_version_sync`, run as a `[[tool.sc-lint.lint]]`
  step, accepts `--root`, `--config` and `--json`, reads a
  `[version_sync.python]` table, and tolerates a missing `Cargo.lock`.
  It also supports atm-core's `[version_sync.winget]` and
  `[version_sync.release_wiring]` subsections (Amended phase L, #137; Accepted —
  Rand, 2026-10-27).

- `REQ-CLI-004`
  The CLI must normalize exit-code behavior across delegated tools.
  See [cli-contract.md](./cli-contract.md) for the planned exit-code mapping.

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
  possible. See [cli-contract.md](./cli-contract.md) for the planned
  top-level error kinds.

- `REQ-CLI-005D`
  Human-readable output must not expose machine-significant information that is
  unavailable through `--json`.
  (Amended phase L, #586, Accepted — Rand, 2026-10-26.) A test must enforce this:
  for each command family, every machine-significant field present in human
  output is present in the `--json` payload.

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

- `REQ-CLI-005H`
  Every non-interactive top-level command family must use the same envelope
  pattern:
  - stable `command` identifier
  - family-owned payload under `data` on success
  - `CliError` under `error` on failure
  - additive diagnostics only, never family-specific top-level envelope keys

- `REQ-CLI-005J`
  Planned business-verdict commands may exit non-zero after successful
  evaluation without reclassifying the result as `CliError`. For the current
  planning line, `sc-lint check interfaces` is such a command: breaking-change
  detection remains a `CommandEnvelope<T>` result with payload-owned verdict
  fields, while `CliError` remains reserved for execution/config/protocol
  failures. `sc-lint lint sc-boundary`, `sc-lint lint sc-portability` and
  `sc-lint lint sc-runtime` are also such commands: each exits 10 when its
  result has `data.status = "fail"`, and the envelope stays `ok: true`.
  (Amended 2026-10-10, Accepted, #200.)

- `REQ-CLI-005I`
  Before a command family is considered implementation-ready, the contract docs
  must record:
  - its stable command-identifier pattern
  - the owner of its success payload shape
  - the applicable top-level error kinds
  - the tests that enforce envelope and error-shape consistency
  Parser-level usage failures that occur before a concrete subcommand path is
  resolved must document the fallback command identifier they use.
  (Amended phase L, #586, Accepted — Rand, 2026-10-26.) The contract matrix in
  `cli-contract.md` must have a row for every command family, including docs,
  init, test, setup, upgrade and lint.ci.consumer, and a test must assert that
  the envelope key set is identical across families.

- `REQ-CLI-006`
  The CLI must support both direct Rust-library dispatch and delegated
  subprocess-based execution during migration periods.

- `REQ-CLI-006A`
  During migration, the top-level CLI may translate canonical `--json`
  requests into backend-specific machine-output flags such as `--format json`,
  but backend-specific flag shapes must not become part of the stable
  top-level contract.

- `REQ-CLI-006B`
  For release `0.1.x`, `sc-lint lint sc-portability` and
  `sc-lint lint sc-runtime` may be integrated through delegated backend
  execution rather than direct top-level crate dependencies. Any promotion to
  direct linkage requires an explicit boundary/architecture update.

## Command Surface Requirements

- `REQ-CLI-007`
  The initial command surface must include:
  - `sc-lint lint <tool>`
  - `sc-lint view <tool>`
  - `sc-lint check`
  - `sc-lint clippy`
  - `sc-lint ci`
  - `sc-lint version`

- `REQ-CLI-007H`
  The planned Phase `C` interface-versioning command surface must use:
  - `sc-lint check interfaces`
  and must preserve `sc-lint version` as the tool-version inspection path
  rather than overloading it for interface-version checks.
  (Amended phase L, #586, Accepted — Rand, 2026-10-26.) Status: planned, not
  implemented. `CheckTarget` offers only native and xwin, and `sc-lint check
  interfaces` is rejected as an unknown target; documentation must not claim
  it exits 0.

- `REQ-CLI-007A`
  The CLI should support first-class execution modes for common developer
  flows where that yields a clearer contract than burying them inside generic
  lint target names.

- `REQ-CLI-007AA`
  For release `0.1.x`, the `view` command family is reserved as a stable
  top-level grouping, but its target inventory is narrower than `lint` and is
  not required to map one-to-one to backend analyzer crates.

- `REQ-CLI-007AB`
  Release-1 `view` targets must be documented individually before exposure and
  may remain backed by repo-local Python/report plumbing until a later phase
  promotes them into a stronger product contract.
  The A.1a bootstrap targets are:
  - `sc-lint view graph`
  - `sc-lint view findings`
  The A.3 implemented stable target is:
  - `sc-lint view findings`

- `REQ-CLI-007F`
  The primary `sc-lint lint <tool>` identifiers must map one-to-one to backend
  crate ownership boundaries. The planned initial analyzer target set is:
  - `sc-boundary`
  - `sc-portability`
  - `sc-runtime`
  and future crate-backed additions should follow the same pattern.
  These three targets accept `sc-lint lint <target> --rule-filter
  <RULE-ID|family>[,...]`, which the CLI forwards unchanged to the backend
  with the resolved `--config` path; the CLI does not interpret selectors.
  `--rule-filter` on any other `lint` target is a usage error (exit 2).
  (Amended 2026-10-10, Accepted, #201.)

- `REQ-CLI-007G`
  Narrower grouped aliases such as `unix-gating` or `runtime-waits` may exist
  as secondary convenience surfaces, but they must not replace the primary
  backend-crate target names in product documentation or contract ownership.

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
  This requirement satisfies the product-level release-1 requirement recorded
  as `REQ-PRODUCT-016A`.

- `REQ-CLI-008`
  The CLI must preserve room for additional grouped subcommands without
  breaking the initial shape.

## Planned Contract Types

- `REQ-CLI-008A`
  The planned CLI contract must explicitly define:
  - `Cli`
  - `Command`
  - `CommandEnvelope<T>`
  - `CliError`

- `REQ-CLI-008B`
  The CLI implementation must define the product-level profile values:
  - `Fast`
  - `Full`
  - `Ci`

- `REQ-CLI-008C`
  The CLI implementation must define both human and JSON output modes for the
  top-level command surface.

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
  result family for non-interactive command execution and must remain
  consistent with the canonical success/failure envelope documented in
  [cli-contract.md](./cli-contract.md).

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
  If `cargo xwin` is installed, both `check.xwin` and `clippy.xwin` should
  join the `full` lint profile, while `fast` remains `xwin`-free to preserve
  low-latency local feedback. ADR-005 supersedes earlier provisional notes
  that limited initial profile membership to `xwin check` alone.

- `REQ-CLI-015`
  `xwin`-backed preflight must not be part of the source-maintainer `ci` lint
  profile because the product relies on real Windows CI runners for
  authoritative validation. Explicit consumer platform selection follows
  REQ-CLI-023 and does not change this source-maintainer profile.
  (Clarified 2026-10-12, Accepted,
  [ADR-017, change 4](./adr/ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts).)

- `REQ-CLI-016`
  `sc-lint compatibility check` must load exactly
  `[tool.sc-lint].minimum_version` from `sc-lint.toml` (or an explicit
  `--config` file), parse it into a validated SemVer boundary type, and compare
  the installed `sc-lint --json version` result semantically. It must not
  require a Cargo workspace, initialize repository logs/reports, or run
  lint/test work.

- `REQ-CLI-017`
  `sc-lint --json version` and `sc-lint --version --json` must emit the stable
  `sc-lint-version-v1` probe with `tool`, SemVer `version`, and `status` fields
  without initializing repository logging or report output.

- `REQ-CLI-018`
  Compatibility failures must use stable error codes and include a cause,
  `just setup`/installer recovery guidance, and the `sc-lint docs setup`
  reference. When available, the JSON details and human message must identify
  the configured minimum version, observed version, binary path, and
  configuration path.

- `REQ-CLI-019`
  `sc-lint setup [--dry-run]` and `sc-lint upgrade [--check] [--dry-run]` must
  load the canonical consumer SemVer floor, select the matching host release,
  verify `checksums.txt` before activation, and atomically replace only the
  managed product binary. A current or newer compatible installation is a
  no-op. The selected immutable release version and the configured SemVer floor
  are distinct values in machine output. Failed activation or post-install
  verification must retain the last working managed binary; if restoration
  itself cannot be verified, the command must report a distinct recovery error
  identifying the manual backup location. This direct binary setup contract is
  unchanged by consumer tool provisioning: ordinary bootstrap setup invokes
  the managed-wheel Python helper using the persisted `.sc-lint/install.json`
  tools object. Exact cargo versions and rustup targets, CARGO_HOME handling,
  inspect/install/verify and non-mutating dry-run follow
  [ADR-017, change 6](./adr/ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts)
  (Accepted, Rand, 2026-10-12); lint execution never installs tools.

- `REQ-CLI-020`
  Installer failures must retain the standard `CliError` envelope and stable
  codes for unsupported platform, unavailable release, checksum mismatch,
  permission failure, and failed post-install version verification. Each must
  identify a cause, recovery action, and `sc-lint docs installation` reference.
  A failed rollback is a separate stable failure because it cannot honestly
  promise that the previous managed installation was restored.

- `REQ-CLI-021`
  The product bootstrap `.sc-lint/bootstrap.py` exposes `setup`, `lint`,
  `test`, `upgrade` and `exec`, requires explicit `--config sc-lint.toml`, and
  delegates to the installed `sc-lint` command. Consumer lint/test entry points follow ADR-016
  as amended. ADR-017 adds bootstrap `exec` through the existing managed
  installation/environment, preserving argv and child exit status without
  re-entering consumer profile dispatch. Bootstrap lint accepts
  `[--fail-fast] [selector] [host|all|linux|macos|windows]` and requires
  `--config sc-lint.toml`. Bootstrap test accepts
  `[--fail-fast|--list] --config sc-lint.toml [selector [args...]]`; its
  options and config precede the selector, and arguments after the selector
  are forwarded verbatim. Lint accepts a selector followed by optional
  platform scope. No setup hooks or interpreter-selection policy is added.
  The product-owned assets remain the source of consumer bootstrap. (Amended
  phase L, [ADR-020 decision 7](./adr/ADR-020-phase-l-atm-core-findings.md).)
  (Amended 2026-10-12, Accepted,
  [ADR-017, changes 4–6](./adr/ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts).)

- `REQ-CLI-022`
  `sc-lint init --just [--check|--dry-run]` must materialize only the
  product-owned `sc-lint.toml`, `Justfile` and `.sc-lint/bootstrap.py` files.
  A current integration is idempotent; `--check` and `--dry-run` do not mutate
  it. A differing existing managed-path file is a structured conflict, never
  an overwrite, and a consumer README is outside the command's write set.
  E.6 owns optional CI workflow generation because it depends on the reusable
  release-verified Action rather than a source-checkout fallback.

- `REQ-CLI-023`
  Consumer execution is explicit: `sc-lint lint --consumer --config
  sc-lint.toml ci` runs the complete configured `[[tool.sc-lint.lint]]` profile,
  and `sc-lint test --config sc-lint.toml` runs the complete configured
  `[[tool.sc-lint.test]]` profile. Each profile item has a unique non-empty
  `name` and a non-empty argv `command` array. They do not discover a source
  checkout; consumer-owned command entries remain governed by ADR-016
  Decision 4. Optional named lint profiles contain ordered step names;
  existing list forms and `--profile all` retain their meanings. Platform
  selection is separate: `--platform-scope host|all|linux|macos|windows`.
  Platform denotes the intended target OS and must match its target triple.
  Only literal cargo check/clippy target commands are translated: native
  targets use cargo, while Linux/macOS to `x86_64-pc-windows-msvc` uses cargo
  xwin. Other cross combinations report explicit unsupported skips; missing
  tools or command failures in supported combinations are failures. Missing
  optional declarations preserve existing behavior. The built-in
  source-maintainer lint plan may run independent steps concurrently and uses
  keep-going behavior: every planned step runs even if another fails. Reports
  retain plan order and identify every failed step by its plan index; the
  command exits non-zero if any step fails. Steps that require Cargo's build
  lock are serialized.
  (Amended in phase L, #195/#196, Accepted — Rand, 2026-10-26.)
  Consumer-configured lint and test profiles run
  independent steps concurrently and fail slow by default: every planned step
  runs even if another fails, the report retains declaration order and
  identifies every failed step by its index, and the command exits non-zero if
  any step fails. The `--fail-fast` flag stops scheduling new steps after the
  first failure. Steps that require Cargo's build lock stay serialized, and
  fail-slow holds inside that serialized lane: a failing step there does not
  stop later steps in the lane. A step may declare `after_all = true`, which
  starts it only after every other step has finished. Output
  ordering is deterministic and does not depend on step timing. Consumer test
  profiles support named command and group selectors, `default`, environment,
  working-directory and platform settings, and forwarded command arguments.
  Limitations: selectors are not declared in `install.json`; `default` is one
  selector name, a call accepts one selector, selector descriptions and
  `requires` are not supported, and installer-side selector validation is not
  added. See [ADR-020 decision 7](./adr/ADR-020-phase-l-atm-core-findings.md).
  (Extended 2026-10-12; amended 2026-10-14, Accepted,
  [ADR-017, change 4](./adr/ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts).)

- `REQ-CLI-024`
  `sc-lint docs` must discover the installed offline documentation bundle
  without network access or repository-root discovery. The command must list
  the overview, operator guides, and every package guide; named guides print
  their contents, while `--path` prints the installed bundle (or guide) path
  without mutating the current repository. Bundle and package completeness are
  validated against the release manifest and relative links before packaging.
  A missing or incomplete bundle must return `CLI.SC_LINT_DOCS_UNAVAILABLE`
  with the searched bundle path, recovery action, and `sc-lint docs
  installation` reference.

- `REQ-CLI-025`
  The cargo-shear wrapper must not report as unlinked a Rust file that a
  trybuild harness in the same package consumes through a string-literal
  `.pass(...)` or `.compile_fail(...)` glob, and must keep reporting every
  other unlinked file, empty file and unused dependency as before. No
  per-fixture configuration is required for consumed fixtures.
  (Added phase L, #647. Status: Accepted — Rand, 2026-10-27. See
  [ADR-020](./adr/ADR-020-phase-l-atm-core-findings.md), Sprint l-10.)

- `REQ-CLI-027`
  Consumer profile failures show both child stdout and stderr. When a log
  root is configured, every executed step writes its raw stdout and stderr to
  its own file under `<log-root>/steps/`, named from the step name (made
  filesystem-safe and unique), overwritten each run.
  (Source: #195, Phase M m-3; Status: Accepted — Rand, 2026-10-31
  ("approve"); ADR-022 provides context for the consumer configuration
  surface.)

- `REQ-CLI-028` — **Proposed; Pending Rand acceptance**
  Consumer lint rows only may declare `host = "linux" | "macos" | "windows"` to
  select the operating-system host on which that row's command and literal
  `env` values apply. `host` is independent of `platform`: `platform` continues
  to identify the target OS and, when `target` is present, must match that
  target triple. A row without `host` keeps its existing behavior. A host
  mismatch is an eligibility skip only; it does not choose the command path.
  Once a row passes the host filter, `decide` selects native cargo or cargo
  xwin from the actual host, platform and target.

  Test selectors do not accept `host`; this amendment does not add host
  selection to test rows. At config load, decode and validate the type and
  supported value of `host` on every declared lint row, whether or not the
  row's profile is selected. A non-string or value outside `linux`, `macos`,
  and `windows` fails with the profile-plan config error detail
  `PROFILE.INVALID_HOST`. After load, selection order is: (1) expand the
  selected step/profile membership, (2) validate each selected row's fields
  and host/platform relationship, (3) apply the platform-scope filter, (4)
  apply the optional host filter, and (5) call `decide` for remaining rows.
  The host/platform contradiction check for a no-target row stays in selected
  row validation, before scope filtering; it does not reject an unselected
  row. The scope
  filter matches existing `scope_selects` behavior: default `host` includes
  untagged rows and rows whose
  `platform` matches the actual host; `all` includes every platform row; a
  named scope includes untagged rows and rows tagged for either the actual host
  or named platform. Scope-filtered rows are omitted silently and never appear
  in the plan or report. Therefore `platform = "windows"` rows are dropped
  silently by default host scope on Linux and macOS; the Windows-target xwin
  rows require `--platform-scope all` (or a scope that selects Windows).
  Host selection occurs only after scope: `host_mismatch` can appear only for
  a selected row that survives the scope filter and declares a host different
  from the actual host. The host check runs before `decide`, replacing either
  `unsupported_cross_command` or `unsupported_cross_target` when the host does
  not match. Add `SkipReason::HostMismatch` with stable JSON reason
  `host_mismatch`.

  Host is needed to split rows that share a `platform`/`target` across
  different execution hosts, such as Windows-target clippy on Linux versus
  macOS. For a row without `target`, `platform` already fixes its eligible
  host; `host` is permitted but redundant. A Windows-target row tagged only
  `platform = "windows"` is already omitted by default host scope on Linux and
  macOS, while a row with `host = "linux"` or `"macos"` allows one shared
  Windows target to use a different literal environment on each host when
  scope all selects it. Scope host does not exclude a Windows-target row on a
  Windows developer host. `default_lint_profile` adds the ability to omit a
  CI-only Windows-host row from bare developer lint while the CI profile still
  explicitly selects it; it does not change the Unix host-scope filtering of
  the Linux/macOS xwin rows.

  In `data.steps[]`, `host` always means the actual host. For a row that
  declares a host selector, add `host_selector` to its Run row and to every
  skip row (`host_mismatch`, `unsupported_cross_command`, or
  `unsupported_cross_target`). Keep `host` as the actual host on skips, and
  include it on host-tagged Run rows. Unannotated Run rows keep their existing
  report shape; unannotated cross-skip rows retain their existing actual-host
  `host` field. For example, these are a successful Run row and a
  host-mismatch skip row:

  ```json
  {"name":"clippy-windows-linux","kind":"lint","command":"cargo xwin clippy --all-targets --target x86_64-pc-windows-msvc","status":"pass","platform":"windows","target":"x86_64-pc-windows-msvc","host":"linux","host_selector":"linux"}
  {"name":"clippy-windows-linux","status":"skipped","host":"macos","host_selector":"linux","platform":"windows","target":"x86_64-pc-windows-msvc","reason":"host_mismatch"}
  ```

  `host_selector` is the only new selector-specific report key and appears
  only on rows declaring `host`; do not rename or repurpose `host` or add an
  `actual_host` key. Rows excluded by scope are never reported. A
  host-mismatch skip is neither a failure nor a scheduler input: it does not
  fail `--fail-fast`, stop scheduling, block an `after_all` row, or delay its
  barrier. As today,
  scope-filtered rows are omitted from the returned plan and report before
  execution scheduling; the `after_all` barrier covers only runnable rows
  that pass both filters. Output stays in
  selected declaration/profile order, independent of execution timing.

  **Bare-run default.** Optional `default_lint_profile = "<name>"` at
  `[tool.sc-lint]` applies only to consumer lint. It is a sibling of
  `lint_profiles`: `[[tool.sc-lint.lint]]` is already an array-of-tables, so
  `[tool.sc-lint.lint]` cannot also be a table. A direct `sc-lint lint ci
  --consumer` invocation's `ci` is the required lint target; `--profile NAME`
  is its explicit consumer selector. In the Just/bootstrap interface,
  `just lint [selector [scope]]` uses the first positional argument as selector
  and the second as scope: `just lint ci all` means selector `ci`, scope `all`.
  If neither explicit selector form is supplied, the named default profile is
  selected. Direct `--platform-scope all` without `--profile` still selects
  `default_lint_profile` and applies that scope. In Just, an empty first
  argument leaves the selector unset so the configured default is used with a
  second-slot scope: `just lint "" all`. `just lint ci all` supplies the `ci`
  selector and `all` scope. `--profile all` and Just's positional
  `all` explicitly select every declared lint row; the scope and host filters
  still apply. The Just recipe is documented in
  [just-setup.md](../../docs-bundle/just-setup.md#select-lint-steps-and-run-managed-commands).

  The type of `default_lint_profile` is checked during config load; it must be
  a string. Profile-name resolution is lazy and occurs only when a bare
  consumer lint run uses the default. The value must resolve by exact name in
  `[tool.sc-lint.lint_profiles]`; lint step names are not profiles, so a
  default naming a step fails with `PROFILE.UNKNOWN_DEFAULT_LINT_PROFILE`. An
  empty, unknown, or `"all"` default fails with the same code and reports
  available lint profile names; `"all"` does not get a separate special error.
  A profile declaration named `all` is
  already rejected with the existing `PROFILE.AMBIGUOUS_NAME`. A declared but
  empty profile continues to fail with the existing `PROFILE.EMPTY_MEMBERS`.
  `sc-lint test` and consumer lint with an explicit `--profile` do not resolve
  or fail on an unknown default string.

  Add proposed `ProfilePlanDetailCode::InvalidHost`
  (`PROFILE.INVALID_HOST`) and `ProfilePlanDetailCode::UnknownDefaultLintProfile`
  (`PROFILE.UNKNOWN_DEFAULT_LINT_PROFILE`). A host value that is non-string or
  unsupported fails with `PROFILE.INVALID_HOST` at config load for every
  declared lint row, including rows outside the selected profile. A host that
  contradicts `platform` on a row without `target` fails with
  `PROFILE.INVALID_HOST` during selected-member validation, before platform
  scope filtering; this relationship check applies only to selected rows. A
  host without `platform` filters by host alone. With a `target`, host and
  target platform may differ, enabling cross-host rows. The config key also
  needs to be admitted in `SC_LINT_KEYS` and represented in
  `ScLintConfigFile` when implemented.

  **Worked example (atm-core).** Use this consumer configuration. It uses a
  unique name for every row. The Windows-host Windows-target row is a distinct
  CI-only `--all-targets --target` check; the native Windows row remains a
  separate default-target check for local development.
  `just lint ci all` is shorthand for selecting profile `ci` with scope `all`;
  the direct equivalent is shown after the TOML.

  ```toml
  [tool.sc-lint]
  default_lint_profile = "local"

  [tool.sc-lint.lint_profiles]
  local = ["atm-core-clippy-linux", "atm-core-clippy-macos", "atm-core-clippy-native-windows"]
  ci = ["atm-core-clippy-linux", "atm-core-clippy-macos", "atm-core-clippy-native-windows", "atm-core-clippy-windows-linux", "atm-core-clippy-windows-macos", "atm-core-clippy-windows"]

  [[tool.sc-lint.lint]]
  name = "atm-core-clippy-linux"
  command = ["cargo", "clippy"]
  platform = "linux"
  env = { PYO3_PYTHON = ".bootstrap-venv/bin/python" }

  [[tool.sc-lint.lint]]
  name = "atm-core-clippy-macos"
  command = ["cargo", "clippy"]
  platform = "macos"
  env = { PYO3_PYTHON = ".bootstrap-venv/bin/python" }

  [[tool.sc-lint.lint]]
  name = "atm-core-clippy-native-windows"
  command = ["cargo", "clippy"]
  platform = "windows"
  env = { PYO3_PYTHON = ".bootstrap-venv/Scripts/python.exe" }

  [[tool.sc-lint.lint]]
  name = "atm-core-clippy-windows-linux"
  command = ["cargo", "clippy", "--all-targets"]
  platform = "windows"
  target = "x86_64-pc-windows-msvc"
  host = "linux"
  env = { PYO3_PYTHON = ".bootstrap-venv/bin/python" }

  [[tool.sc-lint.lint]]
  name = "atm-core-clippy-windows-macos"
  command = ["cargo", "clippy", "--all-targets"]
  platform = "windows"
  target = "x86_64-pc-windows-msvc"
  host = "macos"
  env = { PYO3_PYTHON = ".bootstrap-venv/bin/python" }

  [[tool.sc-lint.lint]]
  name = "atm-core-clippy-windows"
  command = ["cargo", "clippy", "--all-targets"]
  platform = "windows"
  target = "x86_64-pc-windows-msvc"
  host = "windows"
  env = { PYO3_PYTHON = ".bootstrap-venv/Scripts/python.exe" }
  ```

  `sc-lint lint ci --consumer --profile ci --platform-scope all` runs the two
  Unix-host target rows through cargo xwin and the Windows-host row through
  native cargo. The host filter selects only rows matching the runner. The
  Windows-host row is CI-only, so `default_lint_profile = "local"` excludes it
  from a Windows developer's bare `just lint`; on Linux/macOS, the default
  scope already silently omits the `platform = "windows"` xwin rows. The
  Windows-target check therefore runs in CI on all three operating systems and
  not in a developer's bare lint invocation.
  A host-matching Linux/macOS row targeting a Windows triple other than
  `x86_64-pc-windows-msvc` still skips with `unsupported_cross_target`, because
  `uses_xwin` supports only that target; the host selector does not change this
  decision.

  **Acceptance tests (deterministic).** Use injected host/platform values and
  controlled planner/scheduler inputs; do not rely on sleeps, race timing or
  event-loop ordering:

  1. Verify filter order and reporting: default host scope on Linux/macOS omits
     Windows-platform rows entirely; scope all admits them to host filtering;
     mismatched declared hosts yield `host_mismatch`, never either cross skip;
     matching Linux/macOS hosts choose xwin from actual host; existing untagged
     cross skips retain their current reasons and report shape.
  2. Verify host-tagged plan and JSON rows carry actual-host `host` and
     configured `host_selector`, while untagged Run rows keep their current
     schema. A selected step outside scope has no report row.
  3. Verify host-mismatch skips remain non-failing and non-blocking: an
     `after_all` runnable row is scheduled after all runnable rows, and a
     host-mismatch skip neither blocks it nor trips `--fail-fast`.
  4. Verify default selection for `sc-lint lint ci --consumer` without
     `--profile` and bare `just lint`, explicit `--profile` and Just positional
     selectors, and direct `--platform-scope all` without `--profile`. Verify
     it does not affect `sc-lint test`.
  5. Verify a string unknown default, `"all"`, and a default naming a step
     produce `PROFILE.UNKNOWN_DEFAULT_LINT_PROFILE` only on bare consumer lint;
     the same config with `sc-lint test` or explicit `--profile ci` succeeds.
     An empty named profile continues to produce `PROFILE.EMPTY_MEMBERS`, and
     declaring profile `all` continues to produce `PROFILE.AMBIGUOUS_NAME`.
  6. Verify non-string and unsupported host values produce
     `PROFILE.INVALID_HOST` during config load, even on a lint row outside the
     selected profile. Verify the no-target host/platform mismatch produces
     `PROFILE.INVALID_HOST` during selected-member validation before scope
     filtering, including when the selected row would be scope-filtered.
     Verify a host without platform filters by host alone. Name the cases
     `invalid_host_fails_before_platform_scope_filter`,
     `host_platform_mismatch_without_target_is_invalid`, and
     `host_without_platform_filters_by_host_only`, and
     `invalid_host_on_unselected_profile_fails_at_config_load`.

  **Compatibility.** Existing configurations without these keys are
  unchanged. Before this change, `host` fails as an unknown lint-row field and
  `default_lint_profile` fails as an unknown `[tool.sc-lint]` key. Adopting
  `default_lint_profile` changes that consumer's bare lint run to the
  named profile; explicit selectors are unaffected. Both keys require
  sc-lint 0.8.4 or newer, once accepted and implemented; older versions reject
  them during configuration validation.

  (Source: #822 and #784, Phase 0.8.4. Proposed amendment to [ADR-022](./adr/ADR-022-tools-and-consumer-configuration.md); status remains Pending until Rand accepts in his own words.)

## Contract References

- See [cli-contract.md](./cli-contract.md) for:
  - top-level success envelope
  - `CliError` failure envelope
  - per-command-family contract invariants
  - stable command-identifier patterns
  - backend-to-CLI normalization rules
  - exit-code mapping guidance

## A.1a Implementation Notes

- the A.1a bootstrap implementation lives under `crates/sc-lint/src/`
- the grouped command root, contract, error, render, and logging seams are
  intentionally split before real backend dispatch begins
- the A.1a consistency gate is enforced in `crates/sc-lint/src/tests.rs`
  so later command families must keep using the same top-level envelope and
  `CliError` pattern

## A.1b Implementation Notes

- the A.1b config loader lives in `crates/sc-lint/src/config.rs`
- the first backend dispatch seam lives in `crates/sc-lint/src/dispatch.rs`
- `lint.sc-boundary` is the first real backend-normalized command path
- the remaining command families keep using the reserved-surface pattern until
  their owning sprints land

## A.2 Implementation Notes

- `LintProfile::{Fast, Full, Ci}` and `OutputMode::{Human, Json}` now live in
  `crates/sc-lint/src/cli.rs`
- the A.2 orchestration layer lives in `crates/sc-lint/src/workflow.rs`
- `--json` selects `OutputMode::Json` for every non-interactive command family
  through the same top-level envelope and `CliError` path
- `lint.fast`, `lint.full`, `lint.ci`, `check.native`, `check.xwin`,
  `clippy.native`, `clippy.xwin`, and top-level `ci` are now implemented
- `full` conditionally adds `xwin` preflight only when `cargo xwin` is
  available; `fast` and `ci` remain `xwin`-free
- parse-time usage failures that occur before command-path resolution render
  with the fallback machine identifier `cli.parse_error`
- repo-local wrappers now map onto the CLI-owned profiles:
  - `just lint`
    defaults to `sc-lint lint full`
  - `just lint fast`
    maps to `sc-lint lint fast`
  - `just lint ci`
    maps to `sc-lint lint ci`
  - `just ci`
    maps to top-level `sc-lint ci`

## A.2 Rule-Disable Policy

- A.2 does not introduce top-level `sc-lint` rule-disable flags
- rule disable behavior remains backend-owned rather than profile-owned
- for the current shipped backend path:
  - `sc-lint-boundary`
    keeps its default rule tuning in backend configuration such as
    `crates/sc-lint-boundary/config/defaults.toml`
- for delegated Python-backed checks used by `lint full` and `lint ci`, the
  existing per-tool config behavior remains authoritative until those tools
  migrate behind first-class Rust command paths
- profile orchestration must not silently suppress or rewrite backend rule
  selections

## A.3 Implementation Notes

- the Python Adapter Protocol now lives in `crates/sc-lint/src/python_adapter.rs`
  on the Rust side and `.just/python_adapter.py` for Python utility payloads
- the adapter schema is `sc-lint-python-v1`
- extracted Python-backed utility commands now include:
  - `sc-lint lint line-counts`
  - `sc-lint lint function-length`
  - `sc-lint lint identity-literals`
  - `sc-lint view findings`
- `view graph` remains reserved because the graph-oriented contract is not yet
  stable enough for exposure
- Python adapter failures are mapped by structured `kind`/`message`/`details`
  fields rather than by scraping raw traceback text
