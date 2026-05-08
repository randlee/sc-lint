# sc-lint Architecture

This document defines the high-level product architecture for `sc-lint`.

## Architecture Goals

The `sc-lint` architecture should:

- provide one stable user-facing CLI
- keep backend tools self-contained
- allow mixed Rust and Python implementations during migration
- keep canonical machine policy in structured TOML
- support consumer repositories without reintroducing ATM-specific coupling

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
  - planned top-level CLI crate for command parsing, config loading, and tool
    dispatch
- `sc-lint-directives`
  - shared directive parsing/types
- `sc-lint-attributes`
  - proc-macro attribute surface for `#[sc_lint(...)]`
- `sc-lint-boundary`
  - analyzer CLI and library for boundary and portability rules

## Top-level CLI Role

The top-level `sc-lint` CLI is the intended stable user-facing entry point.

It should own:

- command parsing
- repo-root discovery
- config loading
- output formatting conventions
- exit-code conventions
- dispatch to backend tools

It may dispatch to:

- Rust library APIs
- specialized binaries
- Python utilities during migration periods

## Backend Crate Isolation

Default backend isolation rule:

- backend tool crates do not depend on each other directly

Allowed shared support:

- `sc-lint-directives`
- future shared support crates only after explicit design approval

This means coordination belongs in:

- the top-level `sc-lint` CLI

and not in:

- direct backend crate cross-calls

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

## Repo-local Automation

`sc-lint` currently uses:

- `Justfile`
- `.just/` Python utilities and wrappers

These provide:

- local development gate orchestration
- external tool wrapping
- Python-based utilities that are not yet migrated to Rust

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
- graph/export contract
  - see [docs/sc-lint/graph-schema.md](./sc-lint/graph-schema.md)
- structured boundary definitions ADR
  - see [docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md](./sc-lint/adr/ADR-004-structured-boundary-definitions.md)

## Architecture Management

- This file owns product-level architecture.
- Crate-specific design notes and rule mechanics remain in `docs/sc-lint/`.
- As the top-level CLI lands, this document should be updated to reflect the
  implemented command topology rather than the current planned one.
