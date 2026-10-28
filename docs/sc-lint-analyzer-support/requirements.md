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

## Related Docs

- [architecture.md](./architecture.md)
- [ADR-013](../sc-lint/adr/ADR-013-analyzer-shared-support.md)
- [crate README](../../crates/sc-lint-analyzer-support/README.md)
