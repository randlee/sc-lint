# sc-lint CLI Requirements

This document defines the detailed requirements for the planned top-level
`sc-lint` CLI crate.

Related ADRs:
- [`./adr/ADR-005-cli-profiles-and-xwin-preflight.md`](./adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- [`./adr/ADR-006-ai-first-cli-contract.md`](./adr/ADR-006-ai-first-cli-contract.md)
- [`./adr/ADR-008-sc-observability-logging.md`](./adr/ADR-008-sc-observability-logging.md)

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
  Resolution is per configuration table for every lint and every Python-backed
  lint (line-counts, identity-literals, version-sync): an explicit `--config`
  file first, then the table from `sc-lint.toml` when it has one, otherwise the
  same table from `.just/lint-config.toml`. A table present in `sc-lint.toml`
  is never shadowed by the fallback file, and a repo whose `sc-lint.toml` lacks
  a table that only `.just/lint-config.toml` holds (as in atm-core's kit-rendered
  `sc-lint.toml`) keeps using the fallback table. A lint whose configuration
  table is found in neither file fails with a structured error; it must not
  pass silently with "no active limits". The
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
  lock are serialized. Consumer-configured lint and test profiles remain
  sequential, declaration-ordered, and fail-fast.
  (Amended phase L, #195/#196, Accepted — Rand, 2026-10-26; supersedes the
  preceding sentence.) Consumer-configured lint and test profiles run
  independent steps concurrently and fail slow by default: every planned step
  runs even if another fails, the report retains declaration order and
  identifies every failed step by its index, and the command exits non-zero if
  any step fails. The `--fail-fast` flag stops scheduling new steps after the
  first failure. Steps that require Cargo's build lock stay serialized, and
  fail-slow holds inside that serialized lane: a failing step there does not
  stop later steps in the lane. A step may declare `after_all = true`, which
  starts it only after every other step has finished. Output
  ordering is deterministic and does not depend on step timing. Generic test
  groups are not added. Consumer test selectors and test argument forwarding
  follow [ADR-020 decision 7](./adr/ADR-020-phase-l-atm-core-findings.md).
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
  (Added phase L, #647. Approved by Rand 2026-10-27. See
  [ADR-020](./adr/ADR-020-phase-l-atm-core-findings.md), Sprint l-10.)

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
