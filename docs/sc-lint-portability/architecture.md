# `sc-lint-portability` Architecture

This document records the crate-local architecture summary for
`sc-lint-portability`.

## Role

`sc-lint-portability` owns AST-sensitive portability analysis and its machine
contract.

## Authoritative Architecture Sources

- [../architecture.md](../architecture.md)
- [../sc-lint/adr/ADR-010-portability-scope-and-parity.md](../sc-lint/adr/ADR-010-portability-scope-and-parity.md)
- [../../crates/sc-lint-portability/README.md](../../crates/sc-lint-portability/README.md)

## Boundary Rules

- portability shares the directive model with `sc-lint-directives` so PORT-010
  can honor explicit item-level site allowances under ADR-022 decision 4 and
  ADR-013
- portability ignores boundary directives and has no dependency on
  `sc-lint-attributes`; that proc-macro crate remains a no-op
- portability rule ids remain crate-owned and versioned as part of this crate's
  public machine surface
- production-scope path, env, shell, and `cfg` parity follow-ons extend this
  crate rather than creating a separate portability crate
- the crate may depend on shared support crates, but not on other analyzer
  crates directly

## Related Docs

- [requirements.md](./requirements.md)
- [../sc-lint/crate-architecture.md](../sc-lint/crate-architecture.md)
