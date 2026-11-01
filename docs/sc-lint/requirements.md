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

## Requirement Traceability

The canonical requirement statements are owned by the product and CLI
requirements. This crate-local index makes their application to this package
explicit without defining duplicate requirements:

- `REQ-PRODUCT-001` and `REQ-PRODUCT-002` define the stable CLI and its
  parsing, configuration, output, and exit-code ownership.
- `REQ-PRODUCT-002A` through `REQ-PRODUCT-002DA` define machine-mode and
  envelope stability.
- `REQ-CLI-001` through `REQ-CLI-025` and `REQ-CLI-027` define accepted
  command-specific CLI behavior. `REQ-CLI-026` is a retired historical id and
  is not reused.
- `REQ-CLI-028` is a proposed requirement for host-selected lint rows and
  default lint-profile selection; its status is **Proposed, Pending Rand
  acceptance** and it does not alter the accepted contract yet.
- `REQ-LOG-001` through `REQ-LOG-009` define the logging contract in
  [logging.md](./logging.md).

## Scope Rules

- the crate must remain the canonical top-level entry point for end users
- non-interactive command families must keep the shared `--json` envelope and
  `CliError` contract
- backend-specific machine contracts must be normalized at the top-level CLI
  boundary rather than exposed directly as the stable product surface
- structured logging remains CLI-owned even when execution is delegated to a
  backend binary

## Related Docs

- [architecture.md](./architecture.md)
- [cli-contract.md](./cli-contract.md)
- [logging.md](./logging.md)
- [crate-architecture.md](./crate-architecture.md)
