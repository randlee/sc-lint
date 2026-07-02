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
- `crates/sc-lint-boundary/src/analysis.rs`
- `crates/sc-lint-boundary/src/graph.rs`
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
- one validated dependency-policy container replaces the current raw
  `DependenciesSection` string vectors in `inventory.rs`; it is constructed in
  an explicit post-deserialization validation pass before `BoundaryRecord`
  instances enter any package-policy analysis
- malformed dependency-policy records fail inventory loading immediately,
  including:
  - malformed `package-a -> package-b` forbidden-edge strings
  - duplicate forbidden-edge rows
  - duplicate dependency/dependent package names within one record
  - unknown fields under `[dependencies]`
- direct workspace package-edge analysis lands in a dedicated
  `crates/sc-lint-boundary/src/package_policy.rs` module rather than being
  folded into `manifest_policy.rs`
- `package_policy.rs` reuses `graph::load_metadata(root)` or one equivalent
  shared workspace-metadata loader already owned by `sc-lint-boundary`; `D.1`
  must not add a third independent `cargo_metadata::MetadataCommand`
  invocation path for package-edge analysis
- `SCB-DEPENDENCY-001`, `SCB-DEPENDENCY-002`, and `SCB-DEPENDENCY-003` land as
  stable fail-only rules under one dedicated `dependencies` rule filter and
  the default `analyze_workspace` path; this keeps REQ-SCB-021 explicit by
  separating package dependency policy from both source-graph boundary checks
  and manifest workspace/version hygiene at the operator-facing filter layer
- `analysis.rs` remains the authoritative crate-wide fail-status classifier:
  `ScbDependency001`, `ScbDependency002`, and `ScbDependency003` are added to
  `analysis::finding_is_failure` or one equivalent centralized fail-only
  classifier that drives `FindingsReport.status`
- rule semantics are locked for direct workspace-member edges only:
  - `allowed_dependencies` is the complete allowlist of direct outgoing
    workspace package dependencies for the owner package
  - `allowed_dependents` is the complete allowlist of direct incoming
    workspace package dependents for the owner package
  - direct workspace package edges include `[dependencies]`,
    `[dev-dependencies]`, `[build-dependencies]`, and target-specific
    `target.<triple>.*dependencies` sections when they name another current
    workspace member
  - `allowed_dependents = []` means no external workspace package may directly
    depend on that owner package
  - `forbidden_edges` denies the exact direct edge even when an allowlist would
    otherwise permit it
- third-party crates.io or git dependencies outside the current workspace
  member set remain out of scope for this sprint and do not produce
  package-policy findings
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawDependenciesSection {
    pub(crate) allowed_dependents: Vec<String>,
    pub(crate) allowed_dependencies: Vec<String>,
    pub(crate) forbidden_edges: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ForbiddenPackageEdge {
    pub(crate) from: WorkspacePackageName,
    pub(crate) to: WorkspacePackageName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageDependencyPolicy {
    pub(crate) allowed_dependents: BTreeSet<WorkspacePackageName>,
    pub(crate) allowed_dependencies: BTreeSet<WorkspacePackageName>,
    pub(crate) forbidden_edges: Vec<ForbiddenPackageEdge>,
}

impl RawDependenciesSection {
    pub(crate) fn validate(self, boundary_id: &BoundaryId) -> Result<PackageDependencyPolicy>;
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

impl RuleId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScbCycle001 => "SCB-CYCLE-001",
            Self::ScbCycle002 => "SCB-CYCLE-002",
            Self::ScbCycle003 => "SCB-CYCLE-003",
            Self::ScbBoundary001 => "SCB-BOUNDARY-001",
            Self::ScbBoundary002 => "SCB-BOUNDARY-002",
            Self::ScbBoundary003 => "SCB-BOUNDARY-003",
            Self::ScbCaller001 => "SCB-CALLER-001",
            Self::ScbDependency001 => "SCB-DEPENDENCY-001",
            Self::ScbDependency002 => "SCB-DEPENDENCY-002",
            Self::ScbDependency003 => "SCB-DEPENDENCY-003",
            Self::ScbManifest001 => "SCB-MANIFEST-001",
            Self::ScbManifest002 => "SCB-MANIFEST-002",
        }
    }
}
```

```rust
pub(crate) struct PackagePolicyReport {
    pub(crate) findings: Vec<Finding>,
}

let metadata = graph::load_metadata(root)?;
pub(crate) fn analyze_package_policy(
    root: &Path,
    metadata: &cargo_metadata::Metadata,
    inventory: &BoundaryInventory,
) -> Result<PackagePolicyReport>;
```

```rust
pub enum RuleFilter {
    Cycles,
    Boundaries,
    InternalOnly,
    ForbidExternalImpls,
    Dependencies,
    Manifests,
}
```

```rust
pub(crate) fn finding_is_failure(finding: &Finding) -> bool {
    matches!(
        finding.rule_id,
        RuleId::ScbCycle001
            | RuleId::ScbBoundary001
            | RuleId::ScbBoundary002
            | RuleId::ScbBoundary003
            | RuleId::ScbCaller001
            | RuleId::ScbDependency001
            | RuleId::ScbDependency002
            | RuleId::ScbDependency003
            | RuleId::ScbManifest001
            | RuleId::ScbManifest002
    )
}

#[test]
fn dependency_rule_findings_flip_report_status_to_fail() {
    // Assert FindingsReport.status == Fail for SCB-DEPENDENCY-001/002/003,
    // not just that each finding carries the expected rule id.
}
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
- `crates/sc-lint-boundary/src/graph.rs` is an explicit sprint target because
  `package_policy.rs` reuses `graph::load_metadata(root)` or one equivalent
  shared workspace-metadata loader instead of adding a third independent cargo
  metadata invocation path
- `crates/sc-lint-boundary/src/analysis.rs` is an explicit sprint target
  because the crate-wide fail-status classifier lives there and must be updated
  when `ScbDependency001`, `ScbDependency002`, and `ScbDependency003` are
  introduced
- dependency policy entries are validated once at inventory load and do not
  propagate through rule analysis as unconstrained raw strings
- the sprint doc names the validated dependency-policy container that replaces
  the current raw `Vec<String>` dependency fields and states that it is
  constructed in a post-deserialization validation pass in `inventory.rs`
- malformed `forbidden_edges` entries fail inventory loading with actionable
  error text that identifies the offending boundary record and edge string
- package dependency policy remains structurally distinct from both source-graph
  boundary checks and manifest policy in the operator-visible filter surface:
  `RuleFilter::Dependencies` owns `SCB-DEPENDENCY-001`, `SCB-DEPENDENCY-002`,
  and `SCB-DEPENDENCY-003`, satisfying the REQ-SCB-021 separation requirement
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
- `FindingsReport.status == Fail` is regression-tested for
  `SCB-DEPENDENCY-001`, `SCB-DEPENDENCY-002`, and `SCB-DEPENDENCY-003`, not
  just the emitted `rule_id` values
- direct-only scope is explicit and regression-tested: a transitive-only
  workspace path does not produce a package-policy finding
- direct workspace package edges from `[dependencies]`,
  `[dev-dependencies]`, `[build-dependencies]`, and target-specific dependency
  sections are in scope for this rule family and are regression-tested with at
  least one dev- or build-dependency workspace edge
- third-party crates.io or git dependencies outside the current workspace
  member set are ignored by this rule family and do not produce false
  positives
- `cargo test -p sc-lint-boundary` passes with regression coverage for:
  - allowed direct dependency pass
  - disallowed direct dependency fail with `SCB-DEPENDENCY-001`
  - disallowed direct dependent fail with `SCB-DEPENDENCY-002`
  - forbidden direct edge fail with `SCB-DEPENDENCY-003`
  - empty `allowed_dependents` blocks one incoming workspace edge
  - dev- or build-dependency workspace edge is enforced by the same rule family
  - transitive-only edge remains clean
  - third-party crates.io or git dependency outside the workspace-member set
    ignored
- `sc-lint-boundary analyze --format text`, `sc-lint-boundary analyze --format json`,
  `sc-lint-boundary analyze --rule-filter dependencies`, and
  `sc-lint lint sc-boundary` all surface the new dependency findings through
  the existing command path
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
