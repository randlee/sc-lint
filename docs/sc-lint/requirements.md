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

## Related Docs

- [architecture.md](./architecture.md)
- [cli-contract.md](./cli-contract.md)
- [logging.md](./logging.md)
- [crate-architecture.md](./crate-architecture.md)
