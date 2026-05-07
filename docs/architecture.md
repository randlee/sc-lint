# sc-lint Architecture

This document is the high-level architecture index for the `sc-lint` project.

## Repository Structure

Current primary crates:

- `sc-lint-directives`
  - shared directive parsing
- `sc-lint-attributes`
  - proc-macro attribute surface for `#[sc_lint(...)]`
- `sc-lint-boundary`
  - analyzer CLI and library for boundary and portability rules

## Architecture Model

`sc-lint` is structured as:

- reusable Rust crates for analysis and attributes
- repo-local automation for lint execution
- project documentation under `docs/sc-lint/`

## Detailed Architecture References

- analyzer MVP and crate roles
  - see [docs/sc-lint/mvp.md](./sc-lint/mvp.md)
- roadmap and split strategy
  - see [docs/sc-lint/roadmap.md](./sc-lint/roadmap.md)
- graph/export contract
  - see [docs/sc-lint/graph-schema.md](./sc-lint/graph-schema.md)
- structured boundary definitions ADR
  - see [docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md](./sc-lint/adr/ADR-004-structured-boundary-definitions.md)

## Architecture Management

- This file stays intentionally high level.
- Crate-specific design notes and rule mechanics should remain in
  `docs/sc-lint/`.
