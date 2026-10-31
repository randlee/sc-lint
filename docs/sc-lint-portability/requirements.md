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
- PORT-010 may be suppressed for one annotated item with
  `#[sc_lint(allow(PORT-010), reason = "...")]`; its non-empty literal reason
  records why the item is intentionally Unix-only
- `PORT-010` is the only portability rule supported by site allowances. A
  well-formed `PORT-001` through `PORT-009` or another unsupported `PORT-*` id
  returns `PORTABILITY.UNKNOWN_RULE` instead of silently doing nothing.
- the allowance does not cover sibling items, methods in an annotated impl,
  module contents, or a file; reports with suppressed findings use schema
  version `0.2.0` and include `allowed`, while empty reports remain schema
  version `0.1.0` and omit that property

## Stable consumer interfaces

Rule IDs and this analyzer's JSON output are stable consumer contracts
(`REQ-PRODUCT-002`, [ADR-011](../sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md)).

## Related Docs

- [architecture.md](./architecture.md)
- [../sc-lint/adr/ADR-010-portability-scope-and-parity.md](../sc-lint/adr/ADR-010-portability-scope-and-parity.md)
- [../../crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)
