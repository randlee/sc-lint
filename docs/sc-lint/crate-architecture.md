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
  - `docs/sc-lint/cli-requirements.md`
  - `docs/sc-lint/cli-architecture.md`
  - `docs/sc-lint/cli-contract.md`
- governing boundary:
  - `boundaries/sc-lint/top-level-cli.toml`
- primary Phase A sprints:
  - `A.1a`
  - `A.1b`
  - `A.2`
  - `A.3`
  - `B.3`

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
  - boundary inventory, ownership, manifest-policy, and AST-sensitive boundary
    analysis
- governing docs:
  - `docs/sc-lint/boundary-enforcement-model.md`
  - `docs/sc-lint/boundary-toml-migration.md`
  - `docs/sc-lint/graph-schema.md`
- governing boundary:
  - `boundaries/sc-lint-boundary/boundary-analyzer.toml`
- primary implementation and planning sprints:
  - `A.1b`
  - `A.6`
  - `A.7`
  - `B.2`

### `sc-lint-portability`

- role:
  - shared platform and OS portability rule family
  - planned future shared owner for Windows-path parity, env portability, and
    shell-portability rule families when those checks remain consumer-neutral
- governing docs:
  - `docs/sc-lint/extraction-plan.md`
  - `docs/sc-lint/roadmap.md`
  - `docs/sc-lint/adr/ADR-010-portability-scope-and-parity.md`
- governing boundary:
  - `boundaries/sc-lint-portability/portability-analyzer.toml`
- primary implementation and planning sprints:
  - `A.4`
  - `B.1`

### `sc-lint-runtime`

- role:
  - shared std runtime and concurrency rule family
- governing docs:
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

## Ownership Rules

- backend crates remain self-contained and do not depend on each other directly
- the top-level `sc-lint` CLI coordinates backend execution
- primary lint targets map to crate ownership boundaries:
  - `sc-boundary`
  - `sc-portability`
  - `sc-runtime`

## Current Plan Coverage

This document keeps crate-level ownership, responsibility, and governing
references explicit for every crate touched by the implemented Phase A line and
the currently planned Phase B follow-ons.
