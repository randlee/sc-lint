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
- [docs/sc-lint/adr/ADR-013-analyzer-shared-support.md](./sc-lint/adr/ADR-013-analyzer-shared-support.md)
- [docs/sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md](./sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md)
- [docs/sc-lint/adr/ADR-015-standard-repo-tools-adoption-kit.md](./sc-lint/adr/ADR-015-standard-repo-tools-adoption-kit.md)
- [docs/sc-lint/adr/ADR-016-python-wheel-runtime-and-no-rust-configuration.md](./sc-lint/adr/ADR-016-python-wheel-runtime-and-no-rust-configuration.md)
- [docs/sc-lint/adr/ADR-017-phase-k-consumer-contracts.md](./sc-lint/adr/ADR-017-phase-k-consumer-contracts.md)

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

## Layered Architecture

`sc-lint` is a layered design. From lowest to highest:

| Layer | Role | Members today |
| --- | --- | --- |
| (a) shared types and attributes | types and the `#[sc_lint(...)]` attribute surface shared by every tool | `sc-lint-schema` (shared types), `sc-lint-directives`, `sc-lint-analyzer-support`, `sc-lint-attributes` |
| (b) independent cargo tools | one self-contained analyzer per rule family, each with its own binary and library | `sc-lint-boundary`, `sc-lint-portability`, `sc-lint-runtime` |
| (c) `sc-lint` CLI | the single user-facing command; calls into (b) | `crates/sc-lint` |
| (d) just template | the recommended just command layout, with recommended recipe mappings to (b) and (c) | `packages/sc-lint-adoption` (`.sc-lint/justfile`), bootstrap assets |

Dependency rules:

- a layer depends only on layers below it
- (b) tools depend only on (a); they never depend on each other
  (see [Backend Crate Isolation](#backend-crate-isolation))
- (c) calls into (b), as a library or as the tool's binary, and adds no lint
  rules of its own
- (d) is a recommendation the consumer repository owns and edits. It maps
  recipes onto (b) and (c). `just test` is managed and customized by the
  consumer repository; `sc-lint` supplies patterns and general helper
  functions for it rather than implementing it in (c)

Each change in a phase lands in the lowest layer that can carry it.

## Crate documentation

Crate-specific requirements and architecture are indexed from the [product
requirements](./requirements.md). The pages are:

| Crate | Requirements | Architecture |
| --- | --- | --- |
| `sc-lint` | [requirements](./sc-lint/requirements.md) | [architecture](./sc-lint/architecture.md) |
| `sc-lint-analyzer-support` | [requirements](./sc-lint-analyzer-support/requirements.md) | [architecture](./sc-lint-analyzer-support/architecture.md) |
| `sc-lint-attributes` | [requirements](./sc-lint-attributes/requirements.md) | [architecture](./sc-lint-attributes/architecture.md) |
| `sc-lint-boundary` | [requirements](./sc-lint-boundary/requirements.md) | [architecture](./sc-lint-boundary/architecture.md) |
| `sc-lint-directives` | [requirements](./sc-lint-directives/requirements.md) | [architecture](./sc-lint-directives/architecture.md) |
| `sc-lint-portability` | [requirements](./sc-lint-portability/requirements.md) | [architecture](./sc-lint-portability/architecture.md) |
| `sc-lint-runtime` | [requirements](./sc-lint-runtime/requirements.md) | [architecture](./sc-lint-runtime/architecture.md) |
| `sc-lint-schema` | [requirements](./sc-lint-schema/requirements.md) | [architecture](./sc-lint-schema/architecture.md) |
| `sc-lint-py` | [requirements](./sc-lint-py/requirements.md) | [architecture](./sc-lint-py/architecture.md) |

Supporting surfaces outside the four layers:

- `boundaries/` structured boundary definitions and planning metadata
  (see [Boundary and Planning Data](#boundary-and-planning-data))
- this repository's own `Justfile` and `.just/` development automation
  (see [Repo-local Automation](#repo-local-automation))
- `bindings/sc-lint-py`, a distribution wheel over (b) and (c) that contains
  no lint logic (ADR-016)

### Where the current code departs from the layering

These are recorded facts, not planned work. Changing any of them is a
redesign and is discussed with the project owner before it is documented or
planned.

- (c) carries consumer workflow beyond calling into (b): the `test`, `ci`,
  `clippy`, `check`, `setup`, `upgrade`, `init` and `compatibility`
  commands, with `workflow.rs`, `consumer_integration.rs`, `installer.rs` and
  `python_adapter.rs` behind them.
- ADR-012 assigns "complete test" to installed `sc-lint` (see
  [Top-level CLI Role](#top-level-cli-role)), and the (d) kit justfile
  routes `setup`, `lint`, `test` and `upgrade` through the bootstrap into (c).
  The layering leaves `just test` to the consumer repository, with `sc-lint`
  supplying patterns and general helper functions. The ADR-012 and ADR-016
  amendments of 2026-10-10 (Accepted, #194) remove this departure: the kit
  justfile routes only `setup`, `lint` and `upgrade`, and the installer adds
  sc-lint's recommended `test` recipe outside the managed block only when the
  consumer `Justfile` has none, after which the consumer owns it.
- (c) embeds a copy of the (d) bootstrap assets
  (`crates/sc-lint/assets/bootstrap.py`, included by `installer.rs`),
  byte-identical to `packages/sc-lint-adoption/.sc-lint/bootstrap.py`.

ADR-017's accepted Phase K changes are delivered within these existing
owners: Rust analyzer crates validate and evaluate analyzer/inventory policy;
the top-level Rust CLI selects consumer lint steps and runs them through the
shared scheduler (parallel, fail slow, opt-in `--fail-fast`);
the bootstrap delegates `exec` and installation to the existing managed
product and wheel; and the setup helper provisions only consumer-declared
tools/targets. The installer remains configuration/rendering glue, not a
second Rust policy evaluator. The canonical operator-facing account is in the
offline product guides linked from [configuration](../docs-bundle/configuration.md).

## Planned Interface Versioning Layer

The product capability planned for Phase `C` is `sc-lint-version`, which
treats stable interfaces as versioned artifacts rather than relying on prose
release notes alone.

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

The recorded Phase `C` planning decision is that `sc-lint-version` is a
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
- `sc-lint-py` (`bindings/sc-lint-py`, Phase G, ADR-016)
  - maturin/pyo3 wheel crate exposing a thin binding surface (`version_json`,
    `run(argv)`) over the existing crates and carrying the `sc_lint` Python
    helper package; contains no lint logic and no configuration policy
  - boundary record: `boundaries/sc-lint-py/python-bindings.toml`

Planned later crate:

- `sc-lint-version`
  - planned dedicated workspace crate for multi-family interface versioning
    and baseline artifacts
  - governed by the Phase `C.1` through `C.5` sprint sequence
  - out of current implementation scope until the Phase `C` versioning line
    moves from planning into execution
- `sc-lint-tokio`
  - reserved for Tokio rules that need Tokio dependencies or runtime semantics
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

For consumer repositories, ADR-012 additionally fixes command ownership (this
departs from the layering; see
[Where the current code departs from the layering](#where-the-current-code-departs-from-the-layering)):

- installed `sc-lint` owns setup, compatibility, complete lint, complete test,
  and upgrade behavior (amended 2026-10-10, Accepted, #194: "complete test" is
  dropped; installed `sc-lint` provides `sc-lint test` and a recommended
  `test` recipe, and the consumer owns `just test`)
- `sc-lint init --just` writes only product-owned integration files and never
  a consumer README
- a generated `Justfile` is a thin interface with four public recipes and a
  shared private compatibility preflight
- this repository's root `Justfile` remains the explicit source-maintainer
  path; no command guesses its mode from a directory or Cargo package

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

### Consumer profile implementation boundaries

Consumer test selector decoding lives in
`crates/sc-lint/src/test_selectors.rs`; row/table configuration dispatch lives
in `crates/sc-lint/src/config.rs`. The stable profile error codes and their
configuration error construction live in `crates/sc-lint/src/scheduler.rs`,
alongside the scheduler that reports those errors. The CLI keeps the test
selector and its forwarded arguments in one trailing positional field
(`Command::Test::selector_and_args` in `cli.rs`): this makes every token after
the selector, including tokens beginning with `-`, pass through to the test
command. The public invocation remains
`sc-lint test [--fail-fast|--list] [<selector> [args...]]`; flags precede the
selector and the trailing words are forwarded verbatim.

## Backend Crate Isolation

Default backend isolation rule:

- backend tool crates do not depend on each other directly

Allowed shared support:

- `sc-lint-directives`
- `sc-lint-schema`
- `sc-lint-analyzer-support` for rule-neutral AST scanning, report rendering
  and the `[tool.sc-lint.analyzers.<name>]` configuration parser (ADR-013
  amendment 2026-10-10, Accepted, #201)
- future shared support crates only after explicit design approval

For release `0.2.x`, this means:

- `sc-lint-boundary` and `sc-lint-attributes` may depend on
  `sc-lint-directives` for shared directive parsing/types, and
  `sc-lint-portability` and `sc-lint-runtime` may depend on
  `sc-lint-analyzer-support` for the ADR-013 rule-neutral scanner/rendering
  implementation
- `sc-lint-boundary`, `sc-lint-portability` and `sc-lint-runtime` may depend
  on `sc-lint-analyzer-support` for the rule-neutral analyzer configuration
  parser (ADR-013 amendment 2026-10-10, Accepted, #201)
- `sc-lint-boundary`, `sc-lint-portability`, `sc-lint-runtime`,
  `sc-lint-analyzer-support`, and the top-level `sc-lint` CLI may depend on
  `sc-lint-schema` for the canonical machine-schema types
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
  - current owned extensions:
    - `SCB-CALLER-001` named-caller allowlist enforcement
    - `SCB-DEPENDENCY-001` through `SCB-DEPENDENCY-004` direct workspace
      package-edge policy, including stale configured permissions
    - dedicated operator-visible `dependencies` rule filter, separate from
      both source-graph boundary checks and manifest policy
    - direct-edge scope includes normal, dev, build, and target-specific
      workspace dependency sections
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
    - `SCB-RUNTIME-003`
- `sc-lint-tokio`
  - reserved for Tokio rules that need Tokio dependencies or runtime semantics
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
- package dependency policy
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
That same canonical boundary data is also the planned source for direct
workspace package-edge policy such as `allowed_dependencies`,
`allowed_dependents`, and exact `forbidden_edges`.

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

Consumer repositories do not copy these Python utilities. Their generated
integration delegates exclusively to installed `sc-lint` and the product-owned
`.sc-lint/bootstrap.py` asset as defined by ADR-012.

Phase G extends this consumer-adoption boundary through
[ADR-015](./sc-lint/adr/ADR-015-standard-repo-tools-adoption-kit.md): the
versioned adoption kit is the sole generic installation path. Its runtime
delivery and the prohibition on Rust configuration logic are specified by
[ADR-016](./sc-lint/adr/ADR-016-python-wheel-runtime-and-no-rust-configuration.md).

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

- the `homebrew-publish.yml` workflow's `update-tap` job uses a secondary
  `homebrew-tap/` checkout to rewrite tap-local formula files
- the top-level `sc-lint` formula is the primary supported Homebrew surface
  for normal users
- backend binaries included in the release manifest should be installed from
  that top-level formula when they are part of the supported toolset
- any retained per-backend formula, such as `sc-lint-boundary.rb`, must remain
  explicitly documented as a legacy compatibility surface rather than the
  normal user install path

Phase E adds two release-distribution consumers to that same manifest-led
model: the installed documentation bundle and a reusable GitHub Action. The
root Action is a Node runtime so archive verification and extraction use the
same implementation on Linux, macOS, and Windows. It resolves only the exact
versioned release archive and sibling checksum manifest, verifies SHA-256
before extraction, exposes the binary and `sc-lint-docs` paths, then invokes
the E.1 compatibility preflight before E.3 consumer setup/lint/test. It must
not use `cargo run`, a source checkout, a package manager, or an analyzer
package name as a fallback. Its `v1` major tag identifies the Action interface;
the explicit product-version input selects the immutable release artifact.

This architecture realizes `REQ-PRODUCT-022`; the detailed input, output,
provenance, pinning, and recovery contract is in
`docs/sc-lint/github-action-requirements.md`. No additional ADR is needed:
the Action is another manifest-led release consumer and introduces no new
distribution authority.

## Consumer-Proven Rule Promotion

`sc-lint` should treat some rule families as consumer-proven first and
productized second.

Consumer-proven rule families already shipped in dedicated product crates:

- `sc-lint-portability`: `PORT-004`, `PORT-005`
- `sc-lint-runtime`: `SCB-RUNTIME-001`, `SCB-RUNTIME-002`, `SCB-RUNTIME-003`

Phase K follows Accepted ADR-017: conditional reuse of applicable atm-core
source/tests, consumer-neutral rule semantics in Rust, narrow optional analyzer
policy, and consumer-declared setup data owned by the adoption kit/managed
wheel. The top-level CLI remains the sole consumer lint executor; Python
bindings do not decide lint rules. Final current-facing and offline user docs
are reconciled by k-10 after implementation. See the
[Phase K intent](./plans/phase-k/intent.md) and ADR-017 for the exact scope.

Phase J adds no architecture: dependency upgrades run from a repo-local bd
formula (`.atm-bd/formula/upgrade-<dependency>.formula.toml.j2`, REQ-PRODUCT-025)
poured through the existing atm-bd-orchestration flow. It changes no crate,
CLI contract or public interface, so no ADR governs it (`adrs = ["NONE"]`);
the formula's verdicts, its lead-closed plan gate and the no-agent-merge rule
are requirements (REQ-PRODUCT-026..031), not architecture decisions.
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
- extraction and migration plan
  - see [docs/sc-lint/extraction-plan.md](./sc-lint/extraction-plan.md)
- phase execution plans
  - see [docs/plans/phase-A/foundation-phase-plan.md](./plans/phase-A/foundation-phase-plan.md)
  - see [docs/plans/phase-B/phase-B-plan.md](./plans/phase-B/phase-B-plan.md)
  - see [docs/plans/phase-C/phase-C-plan.md](./plans/phase-C/phase-C-plan.md)
  - see [docs/plans/phase-D/phase-D-plan.md](./plans/phase-D/phase-D-plan.md)
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
- consumer adoption and Just contract ADR
  - see [docs/sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md](./sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md)

## Architecture Management

- This file owns product-level architecture.
- Crate-specific design notes and rule mechanics remain in `docs/sc-lint/`.
- As the top-level CLI lands, this document should be updated to reflect the
  implemented command topology rather than the current planned one.
