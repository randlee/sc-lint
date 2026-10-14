# `sc-lint` Crate Requirements

These requirements define the top-level `sc-lint` crate surface.

## Purpose

The `sc-lint` crate is the stable user-facing CLI package. It owns:

- command parsing
- config loading
- top-level success and error normalization
- backend dispatch
- CLI-owned structured logging

## Authoritative Requirement Sources

The detailed requirement authorities for this crate are:

- [cli-requirements.md](./cli-requirements.md)
- [../requirements.md](../requirements.md)
- [logging.md](./logging.md)

## Scope Rules

- the crate must remain the canonical top-level entry point for end users
- non-interactive command families must keep the shared `--json` envelope and
  `CliError` contract
- backend-specific machine contracts must be normalized at the top-level CLI
  boundary rather than exposed directly as the stable product surface
- structured logging remains CLI-owned even when execution is delegated to a
  backend binary

## Requirement Traceability

The canonical requirement statements are owned by the product and CLI
requirements. This crate-local index makes their application to this package
explicit without defining duplicate requirements:

- `REQ-PRODUCT-001` and `REQ-PRODUCT-002` define the stable CLI and its
  parsing, configuration, output, and exit-code ownership.
- `REQ-PRODUCT-002A` through `REQ-PRODUCT-002DA` define machine-mode and
  envelope stability; `NFR-004` records the cross-cutting stability property.
- `REQ-CLI-001` through `REQ-CLI-024` define command-specific CLI behavior.
- `REQ-LOG-001` through `REQ-LOG-009` are defined canonically in
  [../requirements.md](../requirements.md); [logging.md](./logging.md) is the
  design document that describes their coverage.

## Related Docs

- [architecture.md](./architecture.md)
- [cli-contract.md](./cli-contract.md)
- [logging.md](./logging.md)
- [crate-architecture.md](./crate-architecture.md)
