# `sc-lint` Crate Architecture

This document records the crate-by-crate architectural roles for the current
`sc-lint` plan.

## Purpose

The current plan touches multiple crates with different ownership and migration
timing.
This file is the consolidated crate-level reference that ties those crates to
their governing docs, boundaries, and planned sprint work.

## Crate Map

### `sc-lint`

- role:
  - top-level CLI crate
  - owns command parsing, config loading, top-level output normalization, and
    backend dispatch
- governing docs:
  - `docs/sc-lint/requirements.md`
  - `docs/sc-lint/architecture.md`
  - `docs/sc-lint/cli-requirements.md`
  - `docs/sc-lint/cli-architecture.md`
  - `docs/sc-lint/cli-contract.md`
- governing boundary:
  - `boundaries/sc-lint/top-level-cli.toml`
- primary implementation and planning sprints:
  - `A.1a`
  - `A.1b`
  - `A.2`
  - `A.3`
  - `B.3`
  - `C.10`
  - `F.4a` — crate-private configure transaction (`configure::apply`),
    object-safe `ManagedArtifact` implementations, reviewed-plan apply, and
    marker-owned Just integration. This module is not a public plugin API;
    F.4b must add its workflow artifact through the same staging/rollback path.

### `sc-lint-directives`

- role:
  - shared directive parsing/types support
- governing docs:
  - `docs/architecture.md`
  - `docs/sc-lint/README.md`
- governing boundary:
  - `boundaries/sc-lint-directives/directive-model.toml`
- primary Phase A sprints:
  - existing support surface only
  - no dedicated migration sprint in Phase A

### `sc-lint-schema`

- role:
  - shared machine-schema types for analyzer inputs, outputs, and shared
    interface artifacts
- governing docs:
  - `docs/architecture.md`
  - `docs/sc-lint/README.md`
  - `docs/sc-lint-version/requirements.md`
- governing boundary:
  - `boundaries/sc-lint-schema/schema.toml`
- primary implementation and planning sprints:
  - existing support surface only
  - updated whenever shared machine contracts expand across analyzer or
    interface-versioning surfaces

### `sc-lint-analyzer-support`

- role:
  - shared rule-neutral AST source discovery, scope classification, and
    text-report rendering support for analyzer crates
  - has no rule family, binary, or user-facing lint target
- governing docs:
  - `docs/architecture.md`
  - `docs/sc-lint/adr/ADR-013-analyzer-shared-support.md`
- primary implementation and planning sprints:
  - Phase E closeout ARCH-001

### `sc-lint-attributes`

- role:
  - proc-macro attribute surface for `#[sc_lint(...)]`
- governing docs:
  - `docs/architecture.md`
  - `docs/sc-lint/mvp.md`
- governing boundary:
  - `boundaries/sc-lint-attributes/attribute-surface.toml`
- primary Phase A sprints:
  - existing support surface only
  - updated indirectly when boundary semantics or directive contracts change

### `sc-lint-boundary`

- role:
  - boundary inventory, ownership, manifest-policy, AST-sensitive boundary
    analysis, and the planned package dependency-policy line
- governing docs:
  - `docs/sc-lint-boundary/requirements.md`
  - `docs/sc-lint-boundary/architecture.md`
  - `docs/sc-lint-boundary/boundary-enforcement-model.md`
  - `docs/sc-lint-boundary/boundary-toml-migration.md`
  - `docs/sc-lint-boundary/graph-schema.md`
- governing boundary:
  - `boundaries/sc-lint-boundary/boundary-analyzer.toml`
- primary implementation and planning sprints:
  - `A.1b`
  - `A.6`
  - `A.7`
  - `B.2`
  - `D.1`

### `sc-lint-portability`

- role:
  - shared platform and OS portability rule family
  - planned future shared owner for Windows-path parity, env portability, and
    shell-portability rule families when those checks remain consumer-neutral
  - planned future shared owner for structural cross-platform branch-parity
    rules when those checks remain consumer-neutral
  - recorded Phase `B.1` backlog owner and Phase `C.6`-`C.9` sprint owner for
    that shared portability follow-on line
- governing docs:
  - `docs/sc-lint-portability/requirements.md`
  - `docs/sc-lint-portability/architecture.md`
  - `docs/sc-lint/extraction-plan.md`
  - `docs/sc-lint/roadmap.md`
  - `docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md`
- governing boundary:
  - `boundaries/sc-lint-portability/portability-analyzer.toml`
- primary implementation and planning sprints:
  - `A.4`
  - `B.1`
  - `C.6`
  - `C.7`
  - `C.8`
  - `C.9`

## Current Capability Note — sc-lint-boundary Phase D Line

Phase `D` extends the existing `sc-lint-boundary` crate through sprint `D.1`.

- authoritative planning docs:
  - `docs/plans/phase-D/phase-D-plan.md`
  - `docs/plans/phase-D/sprint-D1.md`
  - `docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md`
- current capability:
  - direct workspace package-edge enforcement from canonical boundary TOML
  - validated `allowed_dependencies`, `allowed_dependents`, and
    `forbidden_edges` policy parsing at inventory load
  - a dedicated package dependency-policy analysis path that stays separate
    from manifest workspace/version hygiene
  - direct-edge scope includes normal, dev, build, and target-specific
    workspace dependency sections
- current rule family:
  - `SCB-DEPENDENCY-001`
  - `SCB-DEPENDENCY-002`
  - `SCB-DEPENDENCY-003`
- current command surface:
  - `sc-lint-boundary analyze`
  - `sc-lint-boundary analyze --rule-filter dependencies`
  - `sc-lint lint sc-boundary`

### `sc-lint-runtime`

- role:
  - shared std runtime and concurrency rule family
- governing docs:
  - `docs/sc-lint-runtime/requirements.md`
  - `docs/sc-lint-runtime/architecture.md`
  - `docs/sc-lint/extraction-plan.md`
  - `docs/sc-lint/roadmap.md`
- governing boundary:
  - `boundaries/sc-lint-runtime/runtime-analyzer.toml`
- primary implementation and planning sprints:
  - `A.5`

### `sc-lint-tokio`

- role:
  - reserved future home for Tokio-specific runtime rules
- governing docs:
  - `docs/sc-lint/roadmap.md`
  - `docs/sc-lint/extraction-plan.md`
- governing boundary:
  - `boundaries/sc-lint-tokio/tokio-analyzer.toml`
- primary Phase A sprints:
  - no implementation sprint in Phase A
  - remains reserved only

### `sc-lint-version`

- role:
  - planned dedicated workspace crate for stable interface-version checks and
    canonical interface artifacts
  - planned owner of the `cargo-semver-checks` translation layer, multi-family
    verdict model, and interface-report baseline workflow metadata
  - planned consumer of a shared `sc-compose`-orbit reporting layer rather
    than owner of a crate-local HTML renderer
- governing docs:
  - `docs/plans/phase-C/phase-C-plan.md`
  - `docs/sc-lint-version/requirements.md`
  - `docs/sc-lint-version/architecture.md`
  - `docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md`
  - `docs/plans/phase-C/sprint-C1.md`
  - `docs/plans/phase-C/sprint-C2.md`
  - `docs/plans/phase-C/sprint-C3.md`
  - `docs/plans/phase-C/sprint-C4.md`
  - `docs/plans/phase-C/sprint-C5.md`
- planned governing boundary:
  - `boundaries/sc-lint-version/version-checker.toml`
- primary Phase C sprints:
  - `C.1`
  - `C.2`
  - `C.3`
  - `C.4`
  - `C.5`

## Ownership Rules

- backend crates remain self-contained and do not depend on each other directly
- portability and runtime may depend on `sc-lint-analyzer-support` for the
  ADR-013 rule-neutral scanner/rendering implementation
- the top-level `sc-lint` CLI coordinates backend execution
- primary lint targets map to crate ownership boundaries:
  - `sc-boundary`
  - `sc-portability`
  - `sc-runtime`

## Planned Capability Note — sc-lint-version Sub-Line

Phase `C` commits `sc-lint-version` as a planned dedicated workspace crate.

- authoritative planning docs:
  - `docs/plans/phase-C/phase-C-plan.md`
  - `docs/plans/phase-C/sprint-C1.md`
  - `docs/plans/phase-C/sprint-C2.md`
  - `docs/plans/phase-C/sprint-C3.md`
  - `docs/plans/phase-C/sprint-C4.md`
  - `docs/plans/phase-C/sprint-C5.md`
  - `docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md`
- planned invocation path:
  - `sc-lint check interfaces`
- planned configuration surface:
  - `[version.families.<family>]` in `sc-lint` config
- planned initial backend decision:
  - `cargo-semver-checks` powers the Rust public API family through a
    `sc-lint-version` translation layer
- planned reporting decision:
  - reusable HTML/XHTML rendering lives outside `sc-lint-version` itself
  - preferred ownership target is the `sc-compose` repo, potentially as a
    dedicated `sc-reporting` capability
- dedicated crate boundary and implementation planning records remain future
  implementation work after the Phase `C` planning line

## Planned Capability Note — sc-lint-portability Sub-Line

Phase `C` also extends the existing `sc-lint-portability` crate through the
planned portability follow-on sprints `C.6` through `C.9`.

- authoritative planning docs:
  - `docs/plans/phase-C/phase-C-plan.md`
  - `docs/plans/phase-C/sprint-C6.md`
  - `docs/plans/phase-C/sprint-C7.md`
  - `docs/plans/phase-C/sprint-C8.md`
  - `docs/plans/phase-C/sprint-C9.md`
  - `docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md`
- planned rule-family additions:
  - `PORT-006` and `PORT-007`
    - production path-literal portability expansion
  - `PORT-008`
    - production environment-variable portability expansion
  - `PORT-009`
    - shell invocation portability expansion
  - `PORT-010`
    - structural `cfg` parity expansion
- planned ownership rule:
  - the Phase `C` portability follow-ons stay inside the existing
    `sc-lint-portability` crate rather than creating a separate crate line

## Planned Phase F Configure Boundary

Phase F changes only the top-level `sc-lint` crate; it does not create a
configuration, wizard, or workflow crate. Its module ownership is:

- `configure` command/context/request/plan: public CLI boundary and F.1/F.2
  versioned JSON contract;
- Python/Wyvern adapter: optional presentation boundary that cannot change
  policy or apply a target-repository write;
- `configure::apply`, `configure::just`, and `configure::legacy`: F.4a's
  private transaction, marked Just integration, and allowlisted migration;
- `configure::reviewed_removals`: F.4a's private plan-validation and
  legacy-removal-allowlist helpers, extracted from the configure apply dispatch
  path to satisfy RULE-003;
- `configure::artifact::ManagedArtifact`: crate-private, object-safe staged
  output interface; no downstream implementation is permitted;
- `configure::workflow::WorkflowYamlArtifact`: F.4b's YAML implementation of
  the same transaction interface, not a second transaction engine.

The agent/apply path depends on F.1/F.2 and may progress without a qualified
Wyvern UI. The human adapter joins the F.3b capability gate and F.3c agent
contract at F.3d. ADR-014 governs the entire configure boundary.
The four F.1 schemas named in
[`configure-schemas.md`](./configure-schemas.md) are the only public contract
authorities; F.2 through F.4b prove their fixtures on Linux, macOS, and
Windows without creating platform-specific configure types.

## Phase Plan Coverage

This document keeps crate-level ownership, responsibility, and governing
references explicit for every crate touched by the implemented Phase `A`
foundation line, the archived Phase `B` and Phase `C` follow-ons, and the
Phase `D` dependency-policy work.
