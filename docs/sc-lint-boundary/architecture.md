# `sc-lint-boundary` Architecture

This document records the crate-local architecture summary for
`sc-lint-boundary`.

## Role

`sc-lint-boundary` owns boundary inventory loading, ownership-policy analysis,
manifest-policy analysis, the queued package dependency-policy analysis line,
and the boundary-rule machine contract.

## Authoritative Architecture Sources

- [boundary-enforcement-model.md](./boundary-enforcement-model.md)
- [boundary-toml-migration.md](./boundary-toml-migration.md)
- [graph-schema.md](./graph-schema.md)
- [../architecture.md](../architecture.md)

## Boundary Rules

- canonical boundary definitions live in TOML under `boundaries/`
- boundary inventory parity is crate-owned here, not in the top-level CLI
- named-caller policy remains boundary metadata interpreted by this crate
- package dependency policy remains boundary metadata interpreted by this crate
- manifest workspace/version hygiene remains a separate rule family from
  package dependency policy even when both use Cargo metadata
- shared schema types may come from `sc-lint-schema`, but other analyzer crates
  must not be direct dependencies

## Analysis Surfaces

`sc-lint-boundary` owns or queues three distinct analysis surfaces:

- source-graph boundary rules
  - cycles
  - `boundary.internal_only`
  - `boundary.forbid_external_impls`
  - named-caller allowlists
- package dependency policy
  - planned first implementation sprint: `D.1`
  - direct workspace package-edge allowlist and forbidden-edge enforcement from
    boundary inventory plus Cargo metadata
- manifest policy
  - workspace-field inheritance
  - internal path dependency version alignment

These surfaces share one CLI/reporting contract but they do not collapse into
one implementation bucket. Package dependency policy belongs with
boundary-inventory enforcement, not with manifest-hygiene checks.

## Related Docs

- [requirements.md](./requirements.md)
- [../sc-lint/crate-architecture.md](../sc-lint/crate-architecture.md)
