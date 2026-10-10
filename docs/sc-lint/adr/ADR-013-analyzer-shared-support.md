# ADR-013 — Analyzer Shared Support

| Field | Value |
| --- | --- |
| ID | ADR-013 |
| Status | Accepted |
| Date | 2026-08-13 |
| Deciders | team-lead, clint |
| Relates to | ARCH-001 / RULE-005 |

## Context

`sc-lint-portability` and `sc-lint-runtime` independently carried equivalent
AST source-discovery and text-report rendering implementations. Keeping these
copies in analyzer crates violates the no-duplicate-logic rule and makes future
scanner changes drift, but a direct dependency between the two analyzers would
violate the backend-isolation rule.

## Decision

Create the published support crate `sc-lint-analyzer-support`. It owns only
rule-neutral AST discovery, scope classification, and text-report rendering.
`sc-lint-portability` and `sc-lint-runtime` may depend on it.

The support crate has no rule identifiers, analyzer CLI, backend dispatch, or
analyzer-specific policy. Analyzer rule ownership, public binaries, and
machine-report contracts remain in their existing crates.

## Consequences

- Source scanning and rendering have one implementation and one testable
  behavior surface.
- The analyzer crates remain siblings: neither depends on the other.
- Shared support additions require this crate's release/doc registration but
  do not create a new user-facing lint target.

## Amendment 2026-10-10 — shared analyzer configuration parser

| Field | Value |
| --- | --- |
| Status | Proposed |
| Source | #201, as narrowed by Rand's 2026-10-10 comment on #201 |

1. Extends "It owns only rule-neutral AST discovery, scope classification, and
   text-report rendering" with: and the rule-neutral analyzer configuration
   parser. The parser reads `[tool.sc-lint.analyzers.<name>]` from the
   `--config` file (default `sc-lint.toml` at the root), checks the common keys
   `enabled`, `reason`, `exclude_rules`, `fail_rules` and `exclude_paths`,
   resolves `--rule-filter` selections, rejects any key that is neither common
   nor declared by the analyzer, and applies the settings to findings. It works
   only from the rule ids, rule families, default fail rules and own keys that
   each analyzer passes in.
2. Replaces "`sc-lint-portability` and `sc-lint-runtime` may depend on it" with:
   `sc-lint-boundary`, `sc-lint-portability` and `sc-lint-runtime` may depend
   on it.
3. The second paragraph stands: the support crate defines no rule ids and no
   analyzer-specific policy.

