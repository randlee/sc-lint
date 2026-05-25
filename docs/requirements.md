# sc-lint Requirements

This document defines the high-level product requirements for `sc-lint`.

Related ADRs:
- [docs/sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md](./sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- [docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md](./sc-lint/adr/ADR-006-ai-first-cli-contract.md)
- [docs/sc-lint/adr/ADR-007-analyzer-crate-partition.md](./sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [docs/sc-lint/adr/ADR-008-sc-observability-logging.md](./sc-lint/adr/ADR-008-sc-observability-logging.md)
- [docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md](./sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)
- [docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md](./sc-lint/adr/ADR-010-portability-scope-and-parity.md)

Related design docs:
- [docs/sc-lint/logging.md](./sc-lint/logging.md)
- [docs/sc-lint/version-requirements.md](./sc-lint/version-requirements.md)

For release `0.1.x`, ADR-005 is the approved cross-target preflight strategy
artifact and supersedes earlier provisional profile/`xwin` rollout notes.

## Product Purpose

`sc-lint` is a standalone lint tool family for Rust repositories. It provides:

- AST-sensitive architectural and boundary linting
- portability linting
- reusable lint attributes
- repo-local lint orchestration
- a stable top-level CLI surface

The product should support both:

- direct use in the `sc-lint` repo itself
- reuse from consumer repositories

## Product Surfaces

`sc-lint` has five product surfaces:

1. top-level CLI
2. backend analyzer/tool crates
3. repo-local automation and wrappers
4. structured boundary definitions and planning metadata
5. development and CI profile orchestration

## Core Product Requirements

### Stable CLI

- `REQ-PRODUCT-001`
  `sc-lint` must provide a stable top-level CLI entry point for end users.

- `REQ-PRODUCT-002`
  The top-level CLI must own command parsing, config loading, output
  normalization, and exit-code normalization.

- `REQ-PRODUCT-002A`
  Every non-interactive top-level CLI command must expose a stable
  machine-readable mode. The canonical top-level machine mode is `--json`.

- `REQ-PRODUCT-002B`
  Command families exposed through the top-level CLI must define stable request
  and response contracts that can be reused outside the CLI entrypoint.

- `REQ-PRODUCT-002C`
  When machine mode is requested, both success and failure paths must remain
  machine-readable and must use a stable contract family rather than falling
  back to prose-only stderr.

- `REQ-PRODUCT-002D`
  Machine-readable failure results must include stable codes or categories and
  enough structured detail for automation to branch and recover. See
  [docs/sc-lint/cli-contract.md](./sc-lint/cli-contract.md) for the planned
  top-level error kinds and code mapping.

- `REQ-PRODUCT-002DA`
  The top-level CLI must define one canonical success/failure envelope family
  for non-interactive commands so backend-native machine contracts can be
  normalized without leaking backend-specific contract drift.

- `REQ-PRODUCT-002E`
  Human-readable output must remain a secondary presentation layer and must not
  contain machine-significant detail that is unavailable through machine mode.

- `REQ-PRODUCT-002F`
  Future interactive or graph-exploration features must remain secondary
  surfaces and must not become the only way to access machine-significant
  information.

- `REQ-PRODUCT-003`
  Specialized backend crates must remain self-contained and must not depend on
  each other directly unless a later design review explicitly approves a shared
  support crate.

- `REQ-PRODUCT-003A`
  The top-level CLI should expose one primary lint target per backend analyzer
  crate so the user-facing lint surface preserves crate ownership boundaries.

- `REQ-PRODUCT-003B`
  Narrower grouped aliases such as rule-subset or profile-oriented names may
  exist, but they must remain secondary surfaces layered on top of the primary
  backend-crate mapping rather than replacing it.

### Backend analyzers and tools

- `REQ-PRODUCT-004`
  `sc-lint-boundary` must remain the home for AST-sensitive boundary analysis.

- `REQ-PRODUCT-004A`
  `sc-lint-portability` must be the home for shared AST-sensitive
  platform/OS portability rules.
  Current A.4 implementation status:
  `PORT-001` through `PORT-005` are assigned to `sc-lint-portability`.

- `REQ-PRODUCT-004AA`
  Future shared portability rules for cross-platform path literals,
  environment-variable portability, shell portability, and structural
  cross-platform branch parity must remain owned by `sc-lint-portability`
  when their semantics are consumer-neutral.

- `REQ-PRODUCT-004AB`
  When `sc-lint` carries an OS-specific path-literal portability rule family
  for one major platform class, parity-companion shared detection for the
  matching opposite platform class should remain in `sc-lint-portability`
  rather than drifting into consumer-local wrappers.

- `REQ-PRODUCT-004AC`
  Repo-specific portability policies or shell conventions must not migrate
  into `sc-lint` unchanged; only generic shared portability rules should land
  in `sc-lint-portability`.

- `REQ-PRODUCT-004B`
  `sc-lint-runtime` must be the home for shared AST-sensitive std
  runtime/concurrency correctness rules.
  The release-1 primary top-level target for this family is
  `sc-lint lint sc-runtime`.

- `REQ-PRODUCT-004C`
  Tokio-specific runtime rules must not land in `sc-lint-runtime` by default;
  they should move into `sc-lint-tokio` when Tokio-specific dependencies or
  semantics justify a dedicated crate.

- `REQ-PRODUCT-005`
  Generic, non-AST-sensitive utilities may remain Python-based when Rust does
  not materially improve correctness or noise characteristics.

- `REQ-PRODUCT-006`
  The product must support mixed implementation backends behind one stable CLI
  surface during migration periods.

- `REQ-PRODUCT-006A`
  Reusable lint families proven first in a consumer repository must have an
  explicit migration path into `sc-lint` when their semantics are
  consumer-neutral.

- `REQ-PRODUCT-006AA`
  Reusable lint families imported from a consumer repo must be assigned to the
  narrowest fitting analyzer crate rather than appended to an unrelated
  catch-all crate.

- `REQ-PRODUCT-006B`
  Consumer-specific policy lints must not migrate into `sc-lint` unchanged;
  only the reusable rule family or a configurable framework may be extracted.

- `REQ-PRODUCT-006C`
  The product should provide a documented cross-target preflight strategy for
  surfacing likely platform-specific compile failures before CI where that can
  be done without requiring native execution on every target platform. For
  release `0.1.x`, that governing strategy artifact is
  [docs/sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md](./sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md).

- `REQ-PRODUCT-006D`
  The initial cross-target preflight target should be Windows via `cargo xwin`
  when that path proves reliable in consumer-repo validation.

- `REQ-PRODUCT-006E`
  When `cargo xwin` is installed, the product should expose `xwin`-backed
  Windows preflight support everywhere it can provide meaningful signal
  without requiring separate manual wiring per tool.

- `REQ-PRODUCT-006F`
  During migration periods, the top-level CLI may translate its canonical
  machine contract to backend-specific machine-output flags or adapters, but
  that translation must not leak backend-specific contract drift into the
  stable user-facing surface.

### Release distribution

- `REQ-PRODUCT-006G`
  The release-distribution metadata surface must support multi-crate publish
  planning and multiple released binaries without requiring a distributor- or
  formula-specific schema fork.

- `REQ-PRODUCT-006H`
  When Homebrew distribution is provided, the primary supported tap entry point
  must be `randlee/tap/sc-lint`, and that install path must expose the
  backend binaries promised by the corresponding release manifest. Legacy
  per-backend formulas may remain only when they are explicitly documented as
  secondary compatibility surfaces rather than the normal user install path.

### Interface versioning and publication

- `REQ-PRODUCT-006I`
  The product should define one version-checking capability that covers stable
  interface families beyond Rust crate APIs alone.

- `REQ-PRODUCT-006J`
  The initial version-checking plan must treat Rust public APIs, stable
  top-level CLI contracts, and RPC/socket interfaces as separate interface
  families with explicit breaking-change rules.

- `REQ-PRODUCT-006K`
  Published interface documentation must be generated from structured data and
  reusable templates rather than hand-written monolithic HTML documents.

- `REQ-PRODUCT-006L`
  Generated published interface report packages must follow the main
  HTML-plus-JSON-sidecar model, with separate XHTML section fragments/panels
  for deeper context and built-in copy actions per panel, so one canonical
  machine-readable source can drive both documentation and hard-fail checks.

- `REQ-PRODUCT-006LA`
  The generated published-interface report path must follow the reusable
  workflow described in
  [docs/sc-lint/interface-reporting-constraints.md](./sc-lint/interface-reporting-constraints.md)
  rather than a repo-specific ad hoc HTML rendering path.

- `REQ-PRODUCT-006M`
  The initial Rust public API version-checking approach should be based on
  `cargo-semver-checks` rather than a new custom semver engine.

- `REQ-PRODUCT-006N`
  The version-checking planning line must include one explicit consuming-repo
  adoption document describing the required repo-side harness, fixtures,
  simulators/transcripts when present, and interface inventory
  responsibilities.

- `REQ-PRODUCT-006O`
  The consuming-repo adoption guidance for `sc-lint-version` must be packaged
  as a repo-local Claude Code skill and advertised through a minimal repo-local
  Claude Code marketplace.

- `REQ-PRODUCT-006P`
  The skill-design sprint and the minimal-marketplace sprint must remain
  separate closures in the planning line.

- `REQ-PRODUCT-006Q`
  The planned interface-versioning command surface must not reuse the existing
  tool-version command. It must use a distinct top-level invocation path and a
  documented configuration surface for interface-family selection and
  baselines.

### Logging and observability

- `REQ-LOG-001`
  The top-level CLI must initialize the `sc-observability` logger at startup
  before command execution begins.

- `REQ-LOG-002`
  The default log root must be `~/sc-lint/logs/<service>/`, with a
  per-lint-system override available through config or CLI flag.

- `REQ-LOG-003`
  File logging must be enabled by default and console logging must remain
  opt-in.

- `REQ-LOG-004`
  Each CLI invocation must log through the structured logging runtime:
  - one entry event carrying the command and effective settings/config used
    for the call
  - one completion event carrying the result/verdict and elapsed time in ms
  - one error event per `CliError`, including the stable error code

- `REQ-LOG-005`
  Backend crates must not initialize the logger; structured logging remains a
  CLI-layer responsibility even when backend execution is delegated.

### Boundary definitions

- `REQ-PRODUCT-007`
  Canonical machine-readable boundary definitions must live in TOML under the
  `boundaries/` directory.

- `REQ-PRODUCT-008`
  `sc-lint` must define its own internal crate/tool boundaries as part of
  product planning and future enforcement.

- `REQ-PRODUCT-009`
  Structured planning metadata for planned boundary items must live alongside
  boundary definitions and remain machine-readable.

- `REQ-PRODUCT-009A`
  For release `0.1.x`, boundary inventory enforcement scope must include:
  - crate/tool boundary surfaces
  - planned top-level CLI contract items recorded as boundary composition roots
  and must exclude repo-local automation/profile orchestration surfaces unless
  a later phase models them explicitly in structured boundary records.

- `REQ-PRODUCT-009B`
  Reserved future analyzer crates may be represented in structured boundary
  records before they are scheduled, but they remain out of inventory-parity
  scope until a scheduled sprint and planned-item mapping are assigned.

### Development gate

- `REQ-PRODUCT-010`
  The repo must provide a default local development lint gate through
  `just lint`.

- `REQ-PRODUCT-011`
  The default local lint gate for this repo must include the repo's own
  analyzer checks when those checks are stable and passing.

- `REQ-PRODUCT-012`
  Advisory/manual lint targets may remain outside the default gate only when
  they are not yet stable enough for default development use.

- `REQ-PRODUCT-012A`
  Cross-target preflight checks may live in a separate lint path before they
  join the default local gate, but the project plan must state the intended
  promotion criteria and expected platform coverage.

- `REQ-PRODUCT-012B`
  ADR-005 supersedes earlier provisional sequencing that treated `cargo xwin
  check` as the only initial profile-promotion candidate. The lighter explicit
  preflight path remains `cargo xwin check`, but release `0.1.x` profile
  semantics may include both `cargo xwin check` and `cargo xwin clippy` in
  `full` when the capability is installed.

- `REQ-PRODUCT-012C`
  `xwin` availability should be capability-detected. When unavailable, the
  product should skip `xwin`-specific preflight paths cleanly rather than
  failing unrelated local lint flows.

- `REQ-PRODUCT-012D`
  The product should define named lint profiles for:
  - `fast`
  - `full`
  - `ci`
  and should treat those profiles as product-level semantics rather than only
  `Justfile` conventions.

- `REQ-PRODUCT-012E`
  If `cargo xwin` is installed, `xwin`-backed Windows preflight should be
  eligible for the `full` lint profile through both `check` and `clippy`
  command paths, while `fast` remains `xwin`-free to preserve low-latency
  local feedback and `ci` continues to exclude `xwin`.

### Extraction and migration

- `REQ-PRODUCT-013`
  Generic lint and view utilities currently proven in a consumer repo should be
  extracted into `sc-lint` on a staged basis.

- `REQ-PRODUCT-014`
  Boundary inventory and manifest-policy logic currently implemented in Python
  should migrate into `sc-lint-boundary`.

- `REQ-PRODUCT-015`
  During the Rust migration, the Python boundary implementation must remain
  available as a parity validator until Rust behavior is proven stable.

- `REQ-PRODUCT-015A`
  Reusable postmortem analyzer families proven in `atm-core` must be recorded
  in the project plan as either:
  - migrate to `sc-lint`
  - keep local to the consumer repo
  - extract only as a configurable framework

- `REQ-PRODUCT-015B`
  The current postmortem imports must distribute as:
  - `PORT-004` and `PORT-005` -> `sc-lint-portability`
  - `SCB-RUNTIME-001` and `SCB-RUNTIME-002` -> `sc-lint-runtime`

- `REQ-PRODUCT-015C`
  The current shared rule-family moves required for release `0.1.x` are:
  - `PORT-001`
  - `PORT-002`
  - `PORT-003`
  - `PORT-004`
  - `PORT-005`
  from `sc-lint-boundary` into `sc-lint-portability`, and:
  - `SCB-RUNTIME-001`
  - `SCB-RUNTIME-002`
  into `sc-lint-runtime`.

### Release 1 objective

- `REQ-PRODUCT-016`
  Release `0.1.x` must establish the stable repo-local lint gate, canonical
  TOML boundaries, the documented top-level CLI contract, and the staged
  extraction/migration path for remaining generic tooling.

- `REQ-PRODUCT-016A`
  Release `0.1.x` must define the relationship between:
  - `sc-lint lint ci`
  - `sc-lint ci`
  so lint-only CI parity and full CI-equivalent execution are not ambiguous.
  The CLI-specific traceability for this requirement lives in
  [docs/sc-lint/cli-requirements.md](./sc-lint/cli-requirements.md) under
  `REQ-CLI-007E`.

- `REQ-PRODUCT-017`
  Canonical `sc-lint` boundary definitions must remain the shared planning and
  lint-enforcement inputs under `boundaries/`. With boundary inventory loading
  now implemented in `sc-lint-boundary`, the current release line treats those
  records as active lint inputs rather than future planning-only data.

- `REQ-PRODUCT-018`
  `sc-lint-boundary` may enforce named-caller allowlist policy for explicitly
  configured restricted symbols when that policy is expressed as structured
  boundary metadata. Detailed schema and inventory-loading behavior for that
  feature lives in [docs/sc-lint/requirements.md](./sc-lint/requirements.md).

## Current Detailed Requirement Areas

- Boundary definition and enforcement requirements
  - see [docs/sc-lint/requirements.md](./sc-lint/requirements.md)
- Structured boundary source migration requirements
  - see [docs/sc-lint/boundary-toml-migration.md](./sc-lint/boundary-toml-migration.md)
- Boundary enforcement model requirements
  - see [docs/sc-lint/boundary-enforcement-model.md](./sc-lint/boundary-enforcement-model.md)
- CLI-specific requirements
  - see [docs/sc-lint/cli-requirements.md](./sc-lint/cli-requirements.md)
  - see [docs/sc-lint/cli-contract.md](./sc-lint/cli-contract.md)
- Extraction and phase execution requirements
  - see [docs/sc-lint/extraction-plan.md](./sc-lint/extraction-plan.md)
  - see [docs/sc-lint/foundation-phase-plan.md](./sc-lint/foundation-phase-plan.md)

## Current Phase Requirements

The current execution phase, Phase `B`, requires:

- phase-plan and sprint-plan hardening for the post-Phase-A follow-on work
  currently scheduled in:
  - `B.1`
  - `B.2`
  - `B.3`
  - `B.4`
  - `sprint-B-homebrew`
- explicit top-level traceability for the new Phase-B product lines:
  - recurring shared lint-gate backlog for:
    - raw identity string literals without named constants
    - `/tmp/` paths without intent comments
    - public API error types exposing `anyhow::Error`
    - duplicated `CrateId` newtypes across workspace crates
    - `clippy::for_kv_map` and similar structural for-loop anti-patterns
    - `pub` visibility exceeding the documented contract surface
    - raw `String` fields used for structured identifiers such as
      `boundary_id`, sprint ids, owner ids, and planning keys
  - portability-scope expansion in `sc-lint-portability`
    - Windows-only path literal parity with the current Unix-only path checks
    - broader cross-platform environment-variable portability rules
    - shell-portability checks for OS-specific shell and command assumptions
  - named-caller allowlist enforcement in `sc-lint-boundary`
  - observability boundary-policy acceptance
  - Homebrew release/distribution planning
- requirements and architecture baselines that match the current active phase
  rather than preserving stale Phase-A-only execution wording
- continued preservation of the Phase-A release-1 foundation decisions already
  accepted for:
  - crate-isolated backends
  - the AI-first top-level CLI contract
  - canonical TOML boundary definitions
  - the default local development lint gate
  - documented `fast` / `full` / `ci` profile semantics

## Requirement Management

- This file owns project-level product requirements.
- Detailed crate, rule-family, CLI, and migration requirements should live
  under `docs/sc-lint/`.
- As new `sc-lint` crates are added, crate-specific requirements should be
  linked here.
