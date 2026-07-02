---
id: D.1
title: Boundary Inventory Dependency Policy Enforcement
status: planned
branch: feature/phase-D-planning
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/phase-D-planning
target: develop
---

# Sprint D.1 — Boundary Inventory Dependency Policy Enforcement

## Goal

- close issue `#73` by enforcing package-level dependency policy already
  modeled in boundary inventory
- reject direct workspace package edges that violate
  `allowed_dependencies`, `allowed_dependents`, or `forbidden_edges`
- make package-level architecture policy mechanically enforceable in
  `sc-lint lint sc-boundary` instead of leaving it as schema-only metadata

## Hard Dependencies

- [docs/requirements.md](../requirements.md), especially `REQ-PRODUCT-007`,
  `REQ-PRODUCT-008`, and `REQ-PRODUCT-009A`
- [docs/sc-lint-boundary/requirements.md](../sc-lint-boundary/requirements.md),
  especially `REQ-SCB-001`, `REQ-SCB-006`, and `REQ-SCB-015` through
  `REQ-SCB-021`
- [docs/sc-lint-boundary/architecture.md](../sc-lint-boundary/architecture.md)
- [docs/sc-lint-boundary/boundary-enforcement-model.md](../sc-lint-boundary/boundary-enforcement-model.md)
- [docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md](../sc-lint/adr/ADR-004-structured-boundary-definitions.md)
- [docs/sc-lint/crate-architecture.md](../sc-lint/crate-architecture.md)
- [docs/architecture.md](../architecture.md)
- [docs/project-plan.md](../project-plan.md)
- [boundaries/sc-lint-boundary/boundary-analyzer.toml](../../boundaries/sc-lint-boundary/boundary-analyzer.toml)
- the landed TOML inventory loader, rule-filter wiring, and Cargo-manifest
  analysis path in `sc-lint-boundary`

## Exact Targets

- `docs/phase-D/phase-D-plan.md`
- `docs/phase-D/sprint-D1.md`
- `docs/sc-lint-boundary/requirements.md`
- `docs/sc-lint-boundary/architecture.md`
- `docs/sc-lint/adr/ADR-004-structured-boundary-definitions.md`
- `docs/sc-lint/crate-architecture.md`
- `docs/architecture.md`
- `docs/project-plan.md`
- `docs/issues-inventory.md`
- `crates/sc-lint-boundary/src/inventory.rs`
- `crates/sc-lint-boundary/src/lib.rs`
- `crates/sc-lint-boundary/src/package_policy.rs`
- `crates/sc-lint-boundary/src/tests.rs`
- `crates/sc-lint-boundary/src/main.rs`
- `crates/sc-lint-boundary/README.md`
- `docs/sc-lint-boundary/boundary-enforcement-model.md`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- boundary TOML dependency policy is parsed into validated package-policy types
  at inventory load rather than flowing through analysis as unconstrained raw
  `String` values
- malformed dependency-policy records fail inventory loading immediately,
  including:
  - malformed `package-a -> package-b` forbidden-edge strings
  - duplicate forbidden-edge rows
  - duplicate dependency/dependent package names within one record
  - unknown fields under `[dependencies]`
- direct workspace package-edge analysis lands in a dedicated
  `crates/sc-lint-boundary/src/package_policy.rs` module rather than being
  folded into `manifest_policy.rs`
- `SCB-DEPENDENCY-001`, `SCB-DEPENDENCY-002`, and `SCB-DEPENDENCY-003` land as
  stable fail-only rules under the existing `boundaries` filter and default
  `analyze_workspace` path
- rule semantics are locked for direct workspace-member edges only:
  - `allowed_dependencies` is the complete allowlist of direct outgoing
    workspace package dependencies for the owner package
  - `allowed_dependents` is the complete allowlist of direct incoming
    workspace package dependents for the owner package
  - `allowed_dependents = []` means no external workspace package may directly
    depend on that owner package
  - `forbidden_edges` denies the exact direct edge even when an allowlist would
    otherwise permit it
- non-workspace dependencies remain out of scope for this sprint and do not
  produce package-policy findings
- `sc-lint-boundary analyze` and `sc-lint lint sc-boundary` both surface the
  new rule family in stable text and JSON output without inventing a separate
  ad hoc command path
- crate README and boundary-enforcement docs include one canonical dependency
  policy example plus operator guidance distinguishing:
  - package-edge policy
  - source-reference policy
  - manifest workspace/version hygiene

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```toml
[dependencies]
allowed_dependents = ["sc-lint"]
allowed_dependencies = ["sc-lint-directives", "sc-lint-schema"]
forbidden_edges = [
  "sc-lint-boundary -> sc-lint-attributes",
  "sc-lint-boundary -> sc-observability",
]
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WorkspacePackageName(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ForbiddenPackageEdge {
    pub(crate) from: WorkspacePackageName,
    pub(crate) to: WorkspacePackageName,
}
```

```rust
#[non_exhaustive]
pub enum RuleId {
    ScbCycle001,
    ScbCycle002,
    ScbCycle003,
    ScbBoundary001,
    ScbBoundary002,
    ScbBoundary003,
    ScbCaller001,
    ScbDependency001,
    ScbDependency002,
    ScbDependency003,
    ScbManifest001,
    ScbManifest002,
}
```

```rust
pub(crate) struct PackagePolicyReport {
    pub(crate) findings: Vec<Finding>,
}

pub(crate) fn analyze_package_policy(
    root: &Path,
    inventory: &BoundaryInventory,
) -> Result<PackagePolicyReport>;
```

## This Sprint Does Not Close

- transitive package reachability analysis; `D.1` is direct-edge enforcement
  only
- policy over third-party crates from crates.io or git dependencies outside the
  current workspace-member set
- warning-only or planning-aware package dependency drift; these findings remain
  fail-only in `D.1`
- a generic Cargo-metadata rule framework shared across multiple analyzers
- inventory-parity missing-item enforcement for `implementation.module`,
  `public.facade`, or `composition.root`

## Acceptance Criteria

- the sprint doc makes the implementation split explicit before coding starts:
  package dependency policy lands in `package_policy.rs`, not in
  `manifest_policy.rs`
- dependency policy entries are validated once at inventory load and do not
  propagate through rule analysis as unconstrained raw strings
- malformed `forbidden_edges` entries fail inventory loading with actionable
  error text that identifies the offending boundary record and edge string
- `SCB-DEPENDENCY-001` fails when a workspace member directly depends on
  another workspace member not listed in the owner record's
  `allowed_dependencies`
- `SCB-DEPENDENCY-002` fails when a workspace member directly depends on an
  owner package that does not list that dependent in `allowed_dependents`
- `SCB-DEPENDENCY-003` fails when a direct workspace edge matches an exact
  `forbidden_edges` entry, even if the same edge would otherwise pass both
  allowlists
- `allowed_dependents = []` is treated as “no external workspace package may
  directly depend on this owner package”
- direct-only scope is explicit and regression-tested: a transitive-only
  workspace path does not produce a package-policy finding
- non-workspace dependencies are ignored by this rule family and do not produce
  false positives
- `cargo test -p sc-lint-boundary` passes with regression coverage for:
  - allowed direct dependency pass
  - disallowed direct dependency fail with `SCB-DEPENDENCY-001`
  - disallowed direct dependent fail with `SCB-DEPENDENCY-002`
  - forbidden direct edge fail with `SCB-DEPENDENCY-003`
  - empty `allowed_dependents` blocks one incoming workspace edge
  - transitive-only edge remains clean
  - non-workspace dependency ignored
- `sc-lint-boundary analyze --format text`, `sc-lint-boundary analyze --format json`,
  and `sc-lint lint sc-boundary` all surface the new dependency findings
  through the existing command path
- `crates/sc-lint-boundary/README.md` and
  `docs/sc-lint-boundary/boundary-enforcement-model.md` both include one
  canonical dependency-policy example plus operator guidance on how package-edge
  policy differs from source-reference and manifest-hygiene checks

## Required Validation

- `cargo build --workspace`
- `cargo test -p sc-lint-boundary`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
