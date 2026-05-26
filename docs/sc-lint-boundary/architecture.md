# `sc-lint-boundary` Architecture

This document records the crate-local architecture summary for
`sc-lint-boundary`.

## Role

`sc-lint-boundary` owns boundary inventory loading, ownership-policy analysis,
manifest-policy analysis, and the boundary-rule machine contract.

## Authoritative Architecture Sources

- [boundary-enforcement-model.md](./boundary-enforcement-model.md)
- [boundary-toml-migration.md](./boundary-toml-migration.md)
- [graph-schema.md](./graph-schema.md)
- [../architecture.md](../architecture.md)

## Boundary Rules

- canonical boundary definitions live in TOML under `boundaries/`
- boundary inventory parity is crate-owned here, not in the top-level CLI
- named-caller policy remains boundary metadata interpreted by this crate
- shared schema types may come from `sc-lint-schema`, but other analyzer crates
  must not be direct dependencies

## Related Docs

- [requirements.md](./requirements.md)
- [../sc-lint/crate-architecture.md](../sc-lint/crate-architecture.md)
