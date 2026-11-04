# `sc-lint-analyzer-support` Requirements

## Purpose

`WorkspaceMetadata::workspace_packages()` exposes a rule-neutral view of each
workspace member's dependency name, rename, kind, target platform and target
name, kind and source path. Analyzer policy such as deciding whether a package
declares Tokio remains in the owning analyzer.

The crate centralizes analyzer mechanics that are independent of rule
semantics.

## Requirement Traceability

- `REQ-PRODUCT-004D` defines the crate's shared scanner, scope, rendering, and
  analyzer-configuration responsibilities.
- `REQ-PRODUCT-003` and `REQ-PRODUCT-004D` prohibit analyzer-to-analyzer
  coupling and keep rule ownership in analyzer crates.

## Scope Rules

- Common config and rule-filter behavior is driven by each analyzer's supplied
  rule IDs, families, defaults, and analyzer-specific keys.
- The crate has no CLI, rule IDs, or analyzer-specific policy.
- Since 0.8.5, the crate owns the shared `AnalyzeOptions` analysis input (root and output
  format). Analyzer crates re-export it; an analyzer that needs extra inputs,
  such as the boundary rule filter, keeps its own options type and converts.
- The crate owns repository-relative source-path normalization and
  root-prefix stripping, so every analyzer reports the same slash-separated
  relative paths.
- The crate owns test-only attribute recognition (`#[test]`, qualified test
  attributes, `cfg(test)` and `cfg(all(test, ...))`); each analyzer decides
  which rules the result exempts.

The three 0.8.5 responsibilities above fit ADR-013 as written: path
normalization belongs to source discovery, test-only attribute recognition to
scope classification, and `AnalyzeOptions` carries no rule ids or analyzer
policy. They need no ADR amendment.

## Related Docs

- [architecture.md](./architecture.md)
- [ADR-013](../sc-lint/adr/ADR-013-analyzer-shared-support.md)
- [crate README](../../crates/sc-lint-analyzer-support/README.md)
