# sc-lint Requirements

This document defines the high-level product requirements for `sc-lint`.

Related ADRs:
- [docs/sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md](./sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- [docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md](./sc-lint/adr/ADR-006-ai-first-cli-contract.md)
- [docs/sc-lint/adr/ADR-007-analyzer-crate-partition.md](./sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [docs/sc-lint/adr/ADR-008-sc-observability-logging.md](./sc-lint/adr/ADR-008-sc-observability-logging.md)

Related design docs:
- [docs/sc-lint/logging.md](./sc-lint/logging.md)

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

- `REQ-PRODUCT-004B`
  `sc-lint-runtime` must be the home for shared AST-sensitive std
  runtime/concurrency correctness rules.

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
  be done without requiring native execution on every target platform.

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
  Each CLI invocation must log entry, completion, and per-error events through
  the structured logging runtime.

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
  The default promotion candidate for cross-target preflight is `cargo xwin
  check`, while `cargo xwin clippy` should remain a stronger non-default path
  until timing and noise are proven acceptable.

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
  eligible for the `full` lint profile, while `fast` remains `xwin`-free to
  preserve low-latency local feedback and `ci` continues to exclude `xwin`.

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
  Canonical `sc-lint` boundary definitions may exist as planning inputs before
  loader migration completes, but they must become lint-enforced once boundary
  inventory loading lands in `sc-lint-boundary`.

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

The current execution phase, Phase `A`, requires:

- a top-level `sc-lint` CLI plan with crate-isolated backends
- an AI-first top-level CLI contract with:
  - canonical `--json` machine mode
  - stable request/response seams
  - structured machine-readable failures
- canonical TOML boundary definitions for the current `sc-lint` crates
- a default local development lint gate that runs the repo's own analyzer
  checks
- documented `fast` / `full` / `ci` profile semantics and the distinction
  between `sc-lint lint ci` and top-level `sc-lint ci`
- a staged migration plan for:
  - generic Python utilities
  - boundary inventory and manifest-policy logic moving into Rust
- a documented partition for newly proven lint families so release `0.1.x`
  does not blur reusable analyzer rules with consumer-local policy
- a documented position on cross-target preflight checks for developer
  confidence versus real multi-platform CI validation

## Requirement Management

- This file owns project-level product requirements.
- Detailed crate, rule-family, CLI, and migration requirements should live
  under `docs/sc-lint/`.
- As new `sc-lint` crates are added, crate-specific requirements should be
  linked here.
