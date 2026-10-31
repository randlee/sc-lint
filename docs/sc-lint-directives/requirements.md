# `sc-lint-directives` Requirements

## Purpose

The crate provides one canonical parse model for supported source directives.

## Requirement Traceability

- `REQ-PRODUCT-003` requires shared analyzer boundaries without direct
  analyzer-to-analyzer dependencies.
- `REQ-SCB-001` through `REQ-SCB-007` define the boundary directives and
  validation behavior consumed by this crate.
- ADR-022 decision 4 adds item-scoped portability syntax
  `allow(PORT-010), reason = "..."`, represented as `Directive::SiteAllow`
  separately from boundary `Allow`. Boundary analysis ignores site allowances
  and portability ignores boundary-scoped directives. The literal reason must
  be non-empty, and the exception never cascades to child items. The only
  portability rule currently supported by this site allowance is `PORT-010`;
  other `PORT-*` ids are well-formed syntax but return
  `PORTABILITY.UNKNOWN_RULE` from the portability analyzer.

## Scope Rules

- The proc macro and analyzers use the same typed directive representation.
- Parsing and syntax validation do not decide analyzer findings.
- The `#[sc_lint(...)]` grammar is backward compatible; additions preserve
  existing valid forms and their interpretation.
- Site allowance grammar is `args := element ("," element)* ","?`, where
  `element := boundary-scoped | allow(rule_id ("," rule_id)* ","?) |
  reason = "literal"` and `rule_id := IDENT ("-" IDENT)* "-" INT`.
  Rule ids are unquoted tokens; allowance order is free and a trailing comma
  is accepted. One `allow(...)` and one non-empty string-literal reason are
  required. Boundary directives remain unchanged and may coexist with a site
  allowance.

### Site allowance grammar and diagnostics

| Input | Result | Recovery |
| --- | --- | --- |
| `allow(PORT-010), reason="x"` | One site allowance. | — |
| `reason="x", allow(PORT-010,),` | Same allowance; order is free and a trailing comma is accepted. | — |
| `allow(PORT-010, PORT-011,), reason="x"` | Parses; unsupported portability ids fail later as `PORTABILITY.UNKNOWN_RULE`. | Use `PORT-010` or remove the unsupported allowance. |
| `allow()` | `DIRECTIVE.SITE_ALLOW.EMPTY_RULES`. | Provide at least one unquoted rule id. |
| `allow(PORT-010)` | `DIRECTIVE.SITE_ALLOW.MISSING_REASON`. | Add `reason = "..."` to the allowance. |
| `allow(PORT-010), reason="  "` | `DIRECTIVE.SITE_ALLOW.EMPTY_REASON`. | Provide a non-empty reason string. |
| `allow("PORT-010"), reason="x"` | `DIRECTIVE.SITE_ALLOW.QUOTED_RULE_ID`. | Write rule ids as identifiers, for example `PORT-010`. |
| `allow(PORT-010, PORT-010), reason="x"` | `DIRECTIVE.SITE_ALLOW.DUPLICATE_RULE_ID`. | List each rule id once. |
| `reason="x"` | `DIRECTIVE.SITE_ALLOW.REASON_WITHOUT_ALLOW`. | Add an `allow(...)` element or remove the reason. |
| Two `allow(...)` elements | `DIRECTIVE.SITE_ALLOW.MULTIPLE_ALLOW`. | Combine all rule ids in one `allow(...)` element. |
| Two `reason` elements | `DIRECTIVE.SITE_ALLOW.DUPLICATE_REASON`. | Provide one reason element. |
| `unknown_element` | `DIRECTIVE.SITE_ALLOW.UNKNOWN_ELEMENT`. | Remove unsupported elements from the attribute. |
| `allow(PORT-), reason="x"` | `DIRECTIVE.SITE_ALLOW.MALFORMED_RULE_ID`. | Write an analyzer prefix and numeric suffix, for example `PORT-010`. |
| `allow(PORT-010), reason=variable` | `DIRECTIVE.SITE_ALLOW.NON_LITERAL_REASON`. | Use a string literal for the reason. |
| `allow(PORT-001), reason="x"` | Parses, then returns `PORTABILITY.UNKNOWN_RULE`; only `PORT-010` is supported. | Use `PORT-010` or remove the unsupported allowance. |
| `allow(SCB-MANIFEST-004), reason="x"` | Parses and is ignored by portability because another analyzer owns the id. | — |
| `boundary.allow("B"), allow(PORT-010), reason="x"` | Both directives parse; boundary syntax and behavior stay unchanged. | — |

Every syntax diagnostic has a stable `DIRECTIVE.SITE_ALLOW.<X>` code, cause,
and actionable recovery. `SiteRuleId` accepts only an analyzer prefix with a
numeric suffix, stores its canonical spelling, and cannot be directly
constructed from outside the crate. `SiteAllowReason` preserves the supplied
literal after rejecting empty or whitespace-only values. The typed
`DirectiveError` is `#[non_exhaustive]` so future error cases do not require
downstream exhaustive construction or matching.

## Related Docs

- [architecture.md](./architecture.md)
- [ADR-004 — Structured boundary definitions](../sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- [ADR-007 — Analyzer-crate partition](../sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- [crate README](../../crates/sc-lint-directives/README.md)
- [boundary requirements](../sc-lint-boundary/requirements.md)
