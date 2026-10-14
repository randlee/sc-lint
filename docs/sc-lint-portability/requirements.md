# `sc-lint-portability` Requirements

These requirements define the crate-local rule ownership for
`sc-lint-portability`.

## Purpose

`sc-lint-portability` is the shared analyzer crate for consumer-neutral
platform and OS portability rules.

## Scope Rules

- shared portability rules land here when their semantics are not repo-local
- Unix/Windows path-literal parity stays crate-owned here
- shared environment-variable portability stays crate-owned here
- shared shell-invocation portability stays crate-owned here
- structural `cfg` parity checks stay crate-owned here
- consumer-specific portability wrappers must not be imported unchanged

## Requirement Traceability

- `REQ-PRODUCT-004A` assigns shared platform and OS portability rules to this
  crate.
- `REQ-PRODUCT-004AA` through `REQ-PRODUCT-004AC` define cross-platform path,
  environment, shell, parity, and consumer-specific policy boundaries.
- `PORT-001` through `PORT-010` are the crate-owned rule contracts; their
  definitions and evidence are in the crate README and implementation docs.
- `NFR-001` and `NFR-002` require first-class platform support and parity.

## Related Docs

- [architecture.md](./architecture.md)
- [../sc-lint/adr/ADR-010-portability-scope-and-parity.md](../sc-lint/adr/ADR-010-portability-scope-and-parity.md)
- [../../crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)
