# `sc-lint-directives` Architecture

## Role

This library owns the shared parsed types, syntax validation, and interpretation
of supported `#[sc_lint(...)]` source directives used by the proc macro and
analyzers.

## Governing ADRs

- [ADR-004 — Structured boundary definitions](../sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [ADR-022 — Tools and consumer configuration](../sc-lint/adr/ADR-022-tools-and-consumer-configuration.md),
  decision 4: the `PORT-010` site allowance
- [REQ-PRODUCT-004E](../requirements.md) (Accepted): the same site allowance
  for `SCB-RUNTIME-*` rules

## Boundary Rules

- Parsing is shared; semantic rule ownership stays with the relevant analyzer.
- `Directive::SiteAllow` carries unquoted rule ids and a mandatory non-empty
  literal reason. The portability analyzer honors `PORT-010`; the runtime
  analyzer honors known `SCB-RUNTIME-*` ids on functions and methods. Each
  analyzer ignores ids owned by another analyzer.
- Unknown-id checks are analyzer-owned: an analyzer passes its prefix and
  known ids to `unknown_owned_rule`. The runtime analyzer reports a miss as
  `DIRECTIVE.SITE_ALLOW.UNKNOWN_RULE`; portability reports
  `PORTABILITY.UNKNOWN_RULE`.
- `is_sc_lint_attribute` is the one place that decides which attribute paths
  are `sc_lint` directives; `DirectiveError::from_diagnostic` maps a site-allowance parse
  diagnostic message to its typed `DIRECTIVE.SITE_ALLOW.*` code and recovery.
- The crate must not depend on the CLI or analyzer implementations.

## Related Docs

- [requirements.md](./requirements.md)
- [crate README](../../crates/sc-lint-directives/README.md)
