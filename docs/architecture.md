# sc-lint Architecture

This document defines the high-level product architecture for `sc-lint`.

Related ADRs:
- [docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md](./sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- [docs/sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md](./sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- [docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md](./sc-lint/adr/ADR-006-ai-first-cli-contract.md)
- [docs/sc-lint/adr/ADR-007-analyzer-crate-partition.md](./sc-lint/adr/ADR-007-analyzer-crate-partition.md)

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

## Current and Planned Crates

Current primary crates:

- `sc-lint`
  - top-level CLI crate for command parsing, config loading, and tool dispatch
- `sc-lint-directives`
  - shared directive parsing/types
- `sc-lint-attributes`
  - proc-macro attribute surface for `#[sc_lint(...)]`
- `sc-lint-boundary`
  - analyzer CLI and library for boundary rules
- `sc-lint-portability`
  - planned analyzer CLI and library for platform/OS portability rules
- `sc-lint-runtime`
  - planned analyzer CLI and library for std runtime/concurrency correctness
    rules

Planned later crate:

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
- `lint.sc-boundary`
  - real backend-normalized success path
  - config loading and logger initialization stay in the top-level CLI
  - `sc-lint-boundary` stays a backend-owned analyzer without logger setup

## Backend Crate Isolation

Default backend isolation rule:

- backend tool crates do not depend on each other directly

Allowed shared support:

- `sc-lint-directives`
- future shared support crates only after explicit design approval

For release `0.1.x`, this means:

- `sc-lint-portability` and `sc-lint-runtime` may depend on
  `sc-lint-directives` when shared directive parsing/types are needed
- the top-level `sc-lint` CLI does not directly depend on
  `sc-lint-portability` or `sc-lint-runtime` in the planned release-1
  integration mode

This means coordination belongs in:

- the top-level `sc-lint` CLI

and not in:

- direct backend crate cross-calls

### Rule-family distribution

The release line should avoid growing large catch-all analyzer crates.

Current intended distribution is:

- `sc-lint-boundary`
  - boundary inventory and ownership rules
  - boundary declarations and attribute-driven boundary policy
- `sc-lint-portability`
  - OS/platform portability rules
  - current planned moves/imports:
    - `PORT-001`
    - `PORT-002`
    - `PORT-003`
    - `PORT-004`
    - `PORT-005`
- `sc-lint-runtime`
  - std runtime/concurrency correctness rules
  - current planned imports:
    - `SCB-RUNTIME-001`
    - `SCB-RUNTIME-002`
- `sc-lint-tokio`
  - future Tokio-specific runtime rules
  - must remain distinct from generic runtime rules

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

At the current phase boundary, these TOML records exist as canonical planning
inputs. Default lint enforcement against them becomes active when boundary
inventory loading is moved into `sc-lint-boundary`.

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
- `LintProfile`
  - `Fast`
  - `Full`
  - `Ci`
- `OutputMode`
  - `Human`
  - `Json`
- `CommandEnvelope`
  - boundary/planning metadata tracks the generic family root name
    `CommandEnvelope`, while the CLI contract documents the generic form
    `CommandEnvelope<T>`
- `CliError`

These types are part of the intended architectural contract even before the
full CLI crate is implemented.

For release `0.1.x`, these planned CLI contract types should also be carried in
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

For release `0.1.x`, these repo-local automation/profile surfaces remain
documented product surfaces but are intentionally out of boundary inventory
enforcement scope unless later modeled as explicit boundary records.

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

Current likely first implementation:

- `cargo xwin check --target x86_64-pc-windows-msvc`

Current likely stronger follow-up path:

- `cargo xwin clippy --target x86_64-pc-windows-msvc -- -D warnings`

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

The expected rollout order is:

1. adopt `xwin check` as the first Windows preflight candidate
2. measure timing and usefulness on consumer repos
3. keep `xwin clippy` as a stronger explicit path until its cost is better
   understood

Profile policy:

- `xwin` participation belongs in local lint profiles, not CI-parity profiles
- `fast` excludes `xwin` to preserve low-latency local feedback
- if installed, `xwin check` and `xwin clippy` may join `full`
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

For release `0.1.x`, the intended architecture is that this repo self-hosts
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
  - see [docs/sc-lint/foundation-phase-plan.md](./sc-lint/foundation-phase-plan.md)
- CLI-specific architecture
  - see [docs/sc-lint/cli-architecture.md](./sc-lint/cli-architecture.md)
- CLI-specific contract
  - see [docs/sc-lint/cli-contract.md](./sc-lint/cli-contract.md)
- graph/export contract
  - see [docs/sc-lint/graph-schema.md](./sc-lint/graph-schema.md)
- structured boundary definitions ADR
  - see [docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md](./sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- CLI/profile/xwin execution-model ADR
  - see [docs/sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md](./sc-lint/adr/ADR-005-cli-profiles-and-xwin-preflight.md)
- AI-first CLI contract ADR
  - see [docs/sc-lint/adr/ADR-006-ai-first-cli-contract.md](./sc-lint/adr/ADR-006-ai-first-cli-contract.md)

## Architecture Management

- This file owns product-level architecture.
- Crate-specific design notes and rule mechanics remain in `docs/sc-lint/`.
- As the top-level CLI lands, this document should be updated to reflect the
  implemented command topology rather than the current planned one.
