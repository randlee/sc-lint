# sc-lint Architecture

This document defines the high-level product architecture for `sc-lint`.

Related ADRs:
- [docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md](./sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- [docs/sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md](./sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- [docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md](./sc-lint/adr/ADR-006-ai-first-cli-contract.md)
- [docs/sc-lint/adr/ADR-007-analyzer-crate-partition.md](./sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [docs/sc-lint/adr/ADR-008-sc-observability-logging.md](./sc-lint/adr/ADR-008-sc-observability-logging.md)
- [docs/sc-lint/adr/ADR-009-observability-boundary-policy.md](./sc-lint/adr/ADR-009-observability-boundary-policy.md)
- [docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md](./sc-lint/adr/ADR-010-portability-scope-and-parity.md)
- [docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md](./sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)

For release `0.2.x`, ADR-005 supersedes earlier provisional profile/`xwin`
rollout notes and is the governing cross-target preflight strategy artifact.

## Architecture Goals

The `sc-lint` architecture should:

- provide one stable user-facing CLI
- make the primary CLI contract machine-first and AI-usable
- keep backend tools self-contained
- allow mixed Rust and Python implementations during migration
- keep canonical machine policy in structured TOML
- support consumer repositories without reintroducing ATM-specific coupling
- support a prove-then-promote flow where reusable lint families can mature in
  a consumer repo before they are backported into `sc-lint`

## Product Topology

The product is organized into five layers:

1. top-level CLI
2. backend crates
3. repo-local Python utilities
4. structured boundary definitions and planning metadata
5. repo-local development/CI automation

## Planned Interface Versioning Layer

The next planned product capability after the current Phase `B` line is
`sc-lint-version`, which treats stable interfaces as versioned artifacts
rather than relying on prose release notes alone.

That planned capability spans three interface families:

- Rust public APIs for all shipped crates
- stable top-level CLI commands and machine contracts
- RPC/socket interfaces when such surfaces exist

The intended interface-artifact model is:

- structured canonical interface data
- JSON sidecars as the machine-readable source of truth
- artifact metadata sufficient for a shared report pipeline to publish:
  - main HTML reports
  - separate XHTML section fragments/panels for section-level deep context
  - built-in per-panel copy actions for canonical JSON payload and canonical
    context text

The current Phase `C` planning decision is that `sc-lint-version` is a
dedicated planned workspace crate invoked from the top-level CLI through
`sc-lint check interfaces`.

Configured interface families are selected through `[version.families]` in
`sc-lint` config. Omitted family tables are outside the run; configured
families with no matching current repo surface remain visible as
`not_present`.

For the Rust public API family, `sc-lint-version` is planned to own one
translation layer that consumes `cargo-semver-checks` machine-readable output
and exit-status semantics into the shared multi-family verdict contract.

## Planned Shared Reporting Layer

Phase `C` also plans one shared report-publishing layer consumed by
`sc-lint-version` and intended to be reusable by future non-lint report
surfaces.

That shared layer is intentionally planned as template- and schema-driven
output, not as a collection of hand-maintained HTML pages.

The preferred ownership target for the reusable reporting layer is the
`sc-compose` repo, potentially as a dedicated `sc-reporting` capability,
because the same XHTML-panel conventions are expected to serve:

- public API reports
- CLI contract reports
- ICD-style RPC/socket reports
- later smoke, integration, DB/query, and state-machine reports

The planned report workflow uses:

- structured canonical data from the producing feature
- `sc-compose render`
- Jinja (`.j2`) templates
- one canonical JSON sidecar
- one main HTML report
- separate XHTML panels with mandatory copy controls

Template selection and override are planned under
`[reporting.templates.<report_kind>]` in `sc-lint` config so consumers can
adopt repo-local template variants without forking the producing feature.

Consumer adoption for this layer is also planned as a product surface:

- one clear adoption document for consuming repos covering harness, fixture,
  simulator/transcript, and normalization responsibilities
- one repo-local Claude Code skill that explains the adoption workflow
- one minimal repo-local marketplace entry that advertises that skill
- separate planning closures for the skill-design surface and the marketplace
  publication surface
- consuming repos should leverage existing CLI testability and simulator
  infrastructure where available instead of rebuilding custom interface
  exercisers

## Current and Planned Crates

Current primary crates:

- `sc-lint`
  - top-level CLI crate for command parsing, config loading, and tool dispatch
- `sc-lint-directives`
  - shared directive parsing/types
- `sc-lint-schema`
  - shared machine-schema types for analyzer inputs/outputs
- `sc-lint-attributes`
  - proc-macro attribute surface for `#[sc_lint(...)]`
- `sc-lint-boundary`
  - analyzer CLI and library for boundary rules
- `sc-lint-portability`
  - analyzer CLI and library for platform/OS portability rules
- `sc-lint-runtime`
  - analyzer CLI and library for std runtime/concurrency correctness rules

Planned later crate:

- `sc-lint-version`
  - planned dedicated workspace crate for multi-family interface versioning
    and baseline artifacts
  - governed by the Phase `C.1` through `C.5` sprint sequence
  - out of current implementation scope until the Phase `C` versioning line
    moves from planning into execution
- `sc-lint-tokio`
  - reserved home for Tokio-specific async/runtime lint rules when Tokio-specific
    dependencies or semantics justify a dedicated crate
  - represented in structured boundary metadata as a reserved future crate
  - out of current parity scope until a future sprint assigns planned items

## Top-level CLI Role

The top-level `sc-lint` CLI is the stable user-facing entry point.

It should own:

- command parsing
- repo-root discovery
- config loading
- output formatting conventions
- exit-code conventions
- dispatch to backend tools
- the canonical top-level machine-readable contract

It may dispatch to:

- Rust library APIs
- specialized binaries
- Python utilities during migration periods

The top-level CLI should standardize on:

- canonical machine mode:
  - `--json`
- stable success/failure contract family
- stable profile naming:
  - `fast`
  - `full`
  - `ci`

The primary `lint` target surface should preserve backend-crate ownership
explicitly. Planned primary mappings are:

- `sc-lint lint sc-boundary`
  - backend owner: `sc-lint-boundary`
- `sc-lint lint sc-portability`
  - backend owner: `sc-lint-portability`
- `sc-lint lint sc-runtime`
  - backend owner: `sc-lint-runtime`

Future crate-backed additions should follow the same rule, for example:

- `sc-lint lint sc-tokio`
  - backend owner: `sc-lint-tokio`

Subset aliases such as `unix-gating` or `runtime-waits` may exist as secondary
surfaces, but they must not replace the primary crate-mapped identifiers in
the product contract.

Backend-specific machine flags may still exist internally during migration, but
the user-facing product contract should not depend on them.

Current implementation status:

- `version`
  - direct CLI-owned success path
- `lint.line-counts`
  - delegated Python-backed utility success path through the stable top-level
    CLI contract
- `lint.identity-literals`
  - delegated Python-backed utility success path through the stable top-level
    CLI contract
- `view.findings`
  - delegated Python-backed utility success path through the stable top-level
    CLI contract
- `lint.sc-boundary`
  - real backend-normalized success path
  - config loading and logger initialization stay in the top-level CLI
  - `sc-lint-boundary` stays a backend-owned analyzer without logger setup
- `lint.sc-portability`
  - real delegated backend-normalized success path
  - top-level `sc-lint` invokes the dedicated `sc-lint-portability` binary
    without adding a direct crate dependency
- `lint.sc-runtime`
  - real delegated backend-normalized success path
  - top-level `sc-lint` invokes the dedicated `sc-lint-runtime` binary
    without adding a direct crate dependency

## Backend Crate Isolation

Default backend isolation rule:

- backend tool crates do not depend on each other directly

Allowed shared support:

- `sc-lint-directives`
- `sc-lint-schema`
- future shared support crates only after explicit design approval

For release `0.2.x`, this means:

- `sc-lint-portability` and `sc-lint-runtime` may depend on
  `sc-lint-directives` when shared directive parsing/types are needed
- `sc-lint-boundary`, `sc-lint-portability`, `sc-lint-runtime`, and the
  top-level `sc-lint` CLI may depend on `sc-lint-schema` for the canonical
  machine-schema types
- the top-level `sc-lint` CLI does not directly depend on
  `sc-lint-portability` or `sc-lint-runtime` in the planned release-1
  integration mode

This means coordination belongs in:

- the top-level `sc-lint` CLI

and not in:

- direct backend crate cross-calls

## Observability Boundary Policy

ADR-008 and ADR-009 define the release-1 observability ownership model.

The current approved observability seams are:

- binary-only entry points:
  - `logging::ObservedCommand`
  - `logging::dispatch_event`
- CLI-owned library seam:
  - `contract::ServiceName`
- shared contract field:
  - `CommandEnvelope.command`

Those seams exist so command identity, service identity, and event metadata can
cross the library/binary split without exposing `sc-observability` runtime
types from backend crates or backend public APIs.

Release `0.2.x` observability dependency policy is:

- the mixed lib+bin `sc-lint` package may keep `sc-observability` in
  `[dependencies]`
- that dependency seam is CLI-owned only by architecture policy
- backend crates remain forbidden from taking direct `sc-observability`
  dependencies
- future direct-linked backends may reuse CLI-owned context and contract data,
  but must not own logger initialization or introduce alternative event-entry
  wrappers without a new ADR

The completed Phase `C.10` maintenance line kept this seam list intact while
moving the CLI package to `sc-observability` `1.1.0`. That maintenance scope
was limited to:

- typestate-compatible logger construction and shutdown
- confirmation that direct top-level `emit(...)` call sites remain on the
  supported `Logger<Running>` public API in `1.1.0`
- one explicit retained-log policy decision, with rotation/pruning/background
  maintenance owned by the logger through `RetainedLogPolicy::default()`
- one explicit no decision on `sc-observe` adoption that remains subordinate
  to the existing CLI-owned boundary policy

### Rule-family distribution

The release line should avoid growing large catch-all analyzer crates.

Current intended distribution is:

- `sc-lint-boundary`
  - boundary inventory and ownership rules
  - boundary declarations and attribute-driven boundary policy
  - planned next boundary rule-family addition:
    - `SCB-CALLER-001` named-caller allowlist enforcement
- `sc-lint-portability`
  - OS/platform portability rules
  - current planned moves/imports:
    - `PORT-001`
    - `PORT-002`
    - `PORT-003`
    - `PORT-004`
    - `PORT-005`
  - planned next shared scope:
    - Windows-only path literal parity companion rules
    - broader environment-variable portability rules
    - shell portability rules for OS-specific shell and command assumptions
    - structural `cfg(unix)` / `cfg(windows)` parity enforcement for
      production code
  - consumer-specific portability wrappers remain out of this crate unless they
    are generalized into shared product rules
- `sc-lint-runtime`
  - std runtime/concurrency correctness rules
  - current owned rules:
    - `SCB-RUNTIME-001`
    - `SCB-RUNTIME-002`
- `sc-lint-tokio`
  - future Tokio-specific runtime rules
  - must remain distinct from generic runtime rules

### Phase B Shared Backlog

Phase `B.1` keeps the following reusable lint families explicitly planned
without claiming current implementation:

- raw identity string literals without named constants
- `/tmp/` paths without intent comments
- public API error types exposing `anyhow::Error`
- duplicated `CrateId` newtypes across workspace crates
- `clippy::for_kv_map` and similar structural for-loop anti-patterns
- `pub` visibility exceeding the documented contract surface
- raw `String` fields used for structured identifiers such as `boundary_id`,
  sprint ids, owner ids, and planning keys

## Boundary and Planning Data

Canonical machine policy should live in:

- `boundaries/`

This includes:

- boundary records
- planning metadata

The current target layout is:

```text
boundaries/
  <owner-package>/
    <boundary>.toml
  planning.toml
```

The repository's own crate/tool surfaces should be represented there as part of
the product architecture, not treated only as future consumer-facing examples.
These TOML records are now both canonical planning inputs and active lint
inputs for the boundary inventory behavior already implemented in
`sc-lint-boundary`.

## Current Canonical Boundary Facades

The current boundary definitions and planned CLI surface explicitly name the
important public facades and implementation types for the release-1 line:

- `BOUNDARY-DirectiveModel`
  - facade: `AttributeInput`
  - implementation type: `AttributeInput`
- `BOUNDARY-ScLintAttributeSurface`
  - facade: `sc_lint`
  - implementation type: `sc_lint`
- `BOUNDARY-ScLintBoundaryAnalyzer`
  - facade: `analyze_workspace`
  - implementation type: `analyze_workspace`
- `BOUNDARY-ScLintPortabilityAnalyzer`
  - facade: `analyze_workspace`
  - implementation type: `analyze_workspace`
- `BOUNDARY-ScLintRuntimeAnalyzer`
  - facade: `analyze_workspace`
  - implementation type: `analyze_workspace`
- `BOUNDARY-ScLintTokioAnalyzer`
  - facade: `analyze_workspace`
  - implementation type: `analyze_workspace`
- `BOUNDARY-ScLintCli`
  - facade: `Cli`
  - implementation type: `Cli`

These definitions are canonical in `boundaries/` and should stay aligned with
the implemented Rust item names as the CLI crate lands.

## Planned CLI Contract Types

To keep the release-1 CLI architecture explicit rather than implicit, the
planned top-level CLI surface should also name these important contract types:

- `Cli`
- `Command`
- `CommandEnvelope`
  - boundary/planning metadata tracks the generic family root name
    `CommandEnvelope`, while the CLI contract documents the generic form
    `CommandEnvelope<T>`
- `CliError`
- `CommandContext`
- `CheckTarget`
  - `Native`
  - `Xwin`
- `ClippyTarget`
  - `Native`
  - `Xwin`
- `WINDOWS_XWIN_TARGET`
  - `x86_64-pc-windows-msvc`

These types are part of the intended architectural contract, and for the
A.1b/A.2 line they already match the implemented CLI crate surfaces.

For release `0.2.x`, these planned CLI contract types should also be carried in
machine-readable boundary/planning metadata as `BOUNDARY-ScLintCli`
composition-root items so future inventory-parity work can reason about them
mechanically.

## AI-First CLI Constraint

The top-level `sc-lint` CLI should follow an AI-first contract model for
non-interactive commands:

- machine-readable mode is normative
- top-level failures stay machine-readable when machine mode is requested
- request/response models stay reusable outside the CLI entrypoint
- human-readable output is secondary and must not contain machine-significant
  information that is missing from machine mode

Future MCP wrappers, if added, should reuse the same business request/response
models rather than translating or reshaping them into a second schema.

The detailed top-level success/error normalization contract is documented in:

- [docs/sc-lint/cli-contract.md](./sc-lint/cli-contract.md)

## Repo-local Automation

`sc-lint` currently uses:

- `Justfile`
- `.just/` Python utilities and wrappers

These provide:

- local development gate orchestration
- external tool wrapping
- Python-based utilities that are not yet migrated to Rust

For release `0.2.x`, these repo-local automation/profile surfaces remain
documented product surfaces but are intentionally out of boundary inventory
enforcement scope unless later modeled as explicit boundary records.

## Release Distribution

For release `0.2.x`, release packaging and distributor updates should remain
driven by one canonical manifest surface:

- `release/publish-artifacts.toml`

That manifest is expected to describe:

- multi-crate publish order
- released binary inventory
- preflight and verification requirements

The planned Homebrew path layers on top of that manifest rather than defining a
parallel release inventory. The intended supported install surface is:

- `brew install randlee/tap/sc-lint`

Architecture consequences:

- the `update-homebrew` workflow may use a secondary `homebrew-tap/` checkout
  to rewrite tap-local formula files
- the top-level `sc-lint` formula is the primary supported Homebrew surface
  for normal users
- backend binaries included in the release manifest should be installed from
  that top-level formula when they are part of the supported toolset
- any retained per-backend formula, such as `sc-lint-boundary.rb`, must remain
  explicitly documented as a legacy compatibility surface rather than the
  normal user install path

## Consumer-Proven Rule Promotion

`sc-lint` should treat some rule families as consumer-proven first and
productized second.

Current planned promotion path from `atm-core`:

- reusable analyzer families to backport into dedicated tool crates:
  - `sc-lint-portability`
    - `PORT-004`
    - `PORT-005`
  - `sc-lint-runtime`
    - `SCB-RUNTIME-001`
    - `SCB-RUNTIME-002`
- consumer-local policy families that stay outside `sc-lint` unless extracted
  as a configurable framework:
  - duplicate semantic string-literal policy
  - fixed-sleep test-hygiene policy
  - triage Turtle consistency policy

This preserves backend generality:

- reusable analyzer logic migrates into `sc-lint`
- consumer-specific governance rules remain local unless their framework is
  worth sharing

## Cross-Target Preflight Strategy

`sc-lint` should distinguish between:

- cross-target compile preflight
- true multi-platform validation

Cross-target compile preflight is intended to run from one host platform and
surface likely drift such as:

- missing `cfg` gates
- platform-specific imports leaking into shared code
- type or signature mismatches in gated modules

ADR-005 is the approved release-1 strategy artifact for this area and
supersedes earlier provisional rollout notes from planning documents.

Current explicit Windows preflight commands:

- `cargo xwin check --target x86_64-pc-windows-msvc`
- `cargo xwin clippy --target x86_64-pc-windows-msvc -- -D warnings`

For direct invocation, `xwin check` remains the lighter first-stop preflight
path and `xwin clippy` remains the stronger companion path.

True multi-platform validation still belongs to real CI runners on the target
platforms because cross-target compile checks do not prove:

- runtime behavior
- integration-test behavior
- linker/toolchain behavior on the real host

The architecture therefore supports:

- optional or staged cross-target checks in the local lint flow
- required native-platform CI validation for authoritative release confidence

`xwin` support should be capability-driven:

- if `cargo xwin` is installed, the tool family should expose Windows
  preflight checks wherever they are meaningful
- if `cargo xwin` is not installed, the tool family should degrade cleanly and
  leave those checks unavailable rather than breaking unrelated lint paths

The expected rollout policy is:

1. keep `xwin` capability-gated rather than making it a CI prerequisite
2. allow `full` to add both `xwin check` and `xwin clippy` when the capability
   is present, as defined by ADR-005
3. keep real Windows CI as the authoritative validation path

Profile policy:

- `xwin` participation belongs in local lint profiles, not CI-parity profiles
- `fast` excludes `xwin` to preserve low-latency local feedback
- if installed, `xwin check` and `xwin clippy` join `full`
- `ci` lint parity should stay aligned to real CI and therefore should not
  depend on `xwin`
- the top-level `ci` command should run lint plus tests, while `lint ci`
  remains lint-only

## Current Development Gate

The default development gate is:

- `just lint`

For this repo, that gate should exercise:

- generic repo health checks
- the repo's own stable analyzer checks

Advisory/manual targets may remain outside the default gate only when they are
not yet stable enough for routine development use.

For release `0.2.x`, the intended architecture is that this repo self-hosts
its own analyzer checks through the default development gate wherever those
checks are stable.

## Interactive Surface Constraint

Future graph exploration or type-graph navigation may add interactive
subsurfaces, but those should remain secondary to the canonical machine
contract.

The architecture should not require:

- TTY parsing for automation
- interactive-only access to machine-significant graph data
- richer interactive payloads that lack a corresponding machine-readable form

## Detailed Architecture References

- analyzer MVP and crate roles
  - see [docs/sc-lint/mvp.md](./sc-lint/mvp.md)
- roadmap and split strategy
  - see [docs/sc-lint/roadmap.md](./sc-lint/roadmap.md)
- current extraction and migration plan
  - see [docs/sc-lint/extraction-plan.md](./sc-lint/extraction-plan.md)
- current phase execution plan
  - see [docs/phase-A/foundation-phase-plan.md](./phase-A/foundation-phase-plan.md)
- CLI-specific architecture
  - see [docs/sc-lint/cli-architecture.md](./sc-lint/cli-architecture.md)
- CLI-specific contract
  - see [docs/sc-lint/cli-contract.md](./sc-lint/cli-contract.md)
- graph/export contract
  - see [docs/sc-lint-boundary/graph-schema.md](./sc-lint-boundary/graph-schema.md)
- structured boundary definitions ADR
  - see [docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md](./sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- CLI/profile/xwin execution-model ADR
  - see [docs/sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md](./sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- AI-first CLI contract ADR
  - see [docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md](./sc-lint/adr/ADR-006-ai-first-cli-contract.md)
- portability scope and parity ADR
  - see [docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md](./sc-lint/adr/ADR-010-portability-scope-and-parity.md)
- interface versioning and published artifacts ADR
  - see [docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md](./sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)

## Architecture Management

- This file owns product-level architecture.
- Crate-specific design notes and rule mechanics remain in `docs/sc-lint/`.
- As the top-level CLI lands, this document should be updated to reflect the
  implemented command topology rather than the current planned one.
