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
  failures.

- `REQ-CLI-005I`
  Before a command family is considered implementation-ready, the contract docs
  must record:
  - its stable command-identifier pattern
  - the owner of its success payload shape
  - the applicable top-level error kinds
  - the tests that enforce envelope and error-shape consistency
  Parser-level usage failures that occur before a concrete subcommand path is
  resolved must document the fallback command identifier they use.

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
  `xwin`-backed preflight must not be part of the `ci` lint profile because
  the product relies on real Windows CI runners for authoritative validation.

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
  identifying the manual backup location.

- `REQ-CLI-020`
  Installer failures must retain the standard `CliError` envelope and stable
  codes for unsupported platform, unavailable release, checksum mismatch,
  permission failure, and failed post-install version verification. Each must
  identify a cause, recovery action, and `sc-lint docs installation` reference.
  A failed rollback is a separate stable failure because it cannot honestly
  promise that the previous managed installation was restored.

- `REQ-CLI-021`
  The product bootstrap asset must expose only `ensure`, `setup`, and
  `upgrade`, require an explicit `--config sc-lint.toml`, and delegate to the
  installed `sc-lint` command. E.3 owns rendering this asset into consumer
  repositories and the generated Justfile contract.

- `REQ-CLI-022`
  `sc-lint init --just [--check|--dry-run]` must materialize only the
  product-owned `sc-lint.toml`, `Justfile`, `.sc-lint/bootstrap`, and Windows
  `.sc-lint/bootstrap.ps1` files.
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
  `name` and a non-empty argv `command` array. These commands do not discover
  a source checkout or execute `.just` scripts.

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
  `sc-lint configure` must provide the product-owned consumer setup and
  replacement path. Its MVP must support an explicit root, non-mutating
  context/plan mode, and a stable `--json` result envelope. It may not use
  interactive presentation as its configuration engine. The first context
  collection is limited to conventional path presence and may not parse
  arbitrary user integrations, scan sources, or run Cargo metadata.

- `REQ-CLI-026`
  The configure request and plan payloads must be versioned typed schemas.
  The request supports explicit minimum-version, lint-family choices, Just
  integration selection, CI selection, and consumer profile argv arrays. The
  plan identifies proposed operations, uninspected integration, conflicts,
  summaries, and manual steps. Later apply semantics add preconditions/digests,
  diffs, and removals. Unknown schema versions and invalid fields fail before
  any repository write with a path-qualified `CliError`.

- `REQ-CLI-027`
  `sc-lint configure --request <path|-> --json` must be a noninteractive
  agent-safe path. The optional Wyvern UI must only collect the same request
  data and render the plan; it cannot bypass validation, bounded context, or a
  later final explicit apply confirmation. Its first page must explain what is
  being set up, the standard `just` developer contract, proposed files, facts
  found, uninspected existing integration, and recommendation reasons.
  Absent/failed UI capability has structured recovery and never blocks JSON
  use.

- `REQ-CLI-028`
  Applying a configure plan must revalidate every source digest, stage and
  syntax-check generated TOML/Just/YAML/JSON assets, and restore prior content
  and permissions if any operation or validation fails. Stale plans,
  unmanaged collisions, transaction failures, and incomplete rollback each
  have distinct stable error codes, recovery action, and documentation link.

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
  - `sc-lint lint identity-literals`
  - `sc-lint view findings`
- `view graph` remains reserved because the graph-oriented contract is not yet
  stable enough for exposure
- Python adapter failures are mapped by structured `kind`/`message`/`details`
  fields rather than by scraping raw traceback text
