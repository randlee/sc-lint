# `sc-lint-analyzer-support` Architecture

## Role

This library provides rule-neutral AST source discovery, scope classification,
text report rendering, and shared analyzer configuration parsing. It does not
define lint rules, rule IDs, or an analyzer command surface.

## Governing ADRs

- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [ADR-013 — Analyzer shared support](../sc-lint/adr/ADR-013-analyzer-shared-support.md)

## Boundary Rules

- Analyzer crates may depend on this crate; this crate must not depend on an
  analyzer or the top-level CLI.
- Rule IDs, rule-specific policy, and backend machine contracts remain owned
  by each analyzer.
- Shared configuration behavior uses only rule data supplied by the analyzer.

## Related Docs

- [requirements.md](./requirements.md)
- [crate README](../../crates/sc-lint-analyzer-support/README.md)
