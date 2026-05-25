---
id: B.2
title: Named Caller Allowlist Enforcement
status: planned
branch: feature/phase-B-sprint-plans
worktree: <repo-worktree>/feature/phase-B-sprint-plans
target: develop
---

# Sprint B.2 — Named Caller Allowlist Enforcement

## Goal

- add a TOML-backed named-caller allowlist rule to `sc-lint-boundary`
- make approved-caller policy enforceable in CI instead of a prose-only review step
- close issue `#29` with production-ready rule, CLI, test, and documentation coverage

## Hard Dependencies

- [docs/sc-lint/sprint-B1.md](./sprint-B1.md)
- [docs/sc-lint/requirements.md](./requirements.md), especially `REQ-SCB-006`, `REQ-SCB-007`, and `REQ-SCB-011`
- [boundaries/sc-lint-boundary/boundary-analyzer.toml](../../boundaries/sc-lint-boundary/boundary-analyzer.toml)
- the landed TOML inventory loader and `syn`-backed reference graph in `sc-lint-boundary`

## Exact Targets

- `crates/sc-lint-boundary/src/inventory.rs`
- `crates/sc-lint-boundary/src/analysis.rs`
- `crates/sc-lint-boundary/src/lib.rs`
- `crates/sc-lint-boundary/src/main.rs`
- `crates/sc-lint-boundary/src/tests.rs`
- `crates/sc-lint-boundary/README.md`
- `docs/sc-lint/boundary-enforcement-model.md`
- `docs/sc-lint/phase-B-plan.md`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- boundary TOML gains a structured `[callers]` section for named-caller
  approvals, with schema validation that rejects malformed symbol rows,
  duplicate entries, and unknown fields
- approved caller symbols and caller identities are parsed into validated
  wrapper types at the inventory boundary rather than flowing through analysis
  as unconstrained raw `String` values
- `SCB-CALLER-001` lands as a fail-only rule in `sc-lint-boundary`, using the
  existing reference graph to enumerate all callers of an approved-list symbol
  and emitting a finding for every caller not listed in TOML
- `references.scope = "outside_owner_crate"` semantics apply to the new rule so
  owner-crate callers are exempt when that scope is selected, while external
  callers remain enforced
- `sc-lint-boundary analyze` and `sc-lint lint sc-boundary` surface the new
  rule in stable text/JSON output without inventing a separate ad hoc command
  path
- crate README and boundary-enforcement docs include one canonical example of a
  named-caller record plus the corresponding failure mode and operator
  expectations

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```toml
[callers]
approved = [
  { symbol = "send::hook::maybe_run_post_send_hook", callers = ["atm_core::service_runtime::LocalServiceRuntime"] },
]
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoundaryRecord {
    pub(crate) id: BoundaryId,
    pub(crate) owner: OwnerId,
    pub(crate) roots: Vec<BoundaryRoot>,
    pub(crate) callers: Option<CallersSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CallersSection {
    pub(crate) approved: Vec<ApprovedCallerEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApprovedCallerEntry {
    pub(crate) symbol: ApprovedSymbol,
    pub(crate) callers: Vec<ApprovedCaller>,
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ApprovedSymbol(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ApprovedCaller(String);
```

```rust
pub enum RuleId {
    ScbCycle001,
    ScbCycle002,
    ScbCycle003,
    ScbBoundary001,
    ScbBoundary002,
    ScbBoundary003,
    ScbCaller001,
    ScbManifest001,
    ScbManifest002,
}
```

```rust
pub(crate) fn analyze_named_callers(
    graph: &GraphExport,
    inventory: &BoundaryInventory,
) -> Vec<Finding>;

fn caller_is_exempt(
    record: &BoundaryRecord,
    callers: &CallersSection,
    caller_package: &str,
    caller_path: &str,
) -> bool;
```

## This Sprint Does Not Close

- any new warning/planning mode for caller policy; unapproved callers remain a
  fail-only violation
- a generic allowlist framework for non-boundary analyzers
- symbol-pattern wildcards; `SCB-CALLER-001` closes exact-symbol,
  exact-caller-name enforcement

## Acceptance Criteria

- malformed `[callers]` rows, duplicate symbol entries, and unknown caller
  fields fail during inventory load
- approved symbols and caller identities are validated once at inventory load
  and do not propagate through rule analysis as unconstrained raw strings
- the sprint doc makes the chosen TOML integration point explicit:
  `[callers]` extends each per-boundary `BoundaryRecord` without relaxing the
  surrounding `#[serde(deny_unknown_fields)]` contract
- `SCB-CALLER-001` exits non-zero when a non-exempt external caller reaches a
  restricted symbol and stays clean when all callers are approved or exempt
- `references.scope = "outside_owner_crate"` exempts owner-crate callers
  without exempting external callers
- `cargo test -p sc-lint-boundary` passes with regression coverage for:
  - approved-caller pass
  - unapproved external caller fail with `SCB-CALLER-001`
  - `outside_owner_crate` owner-crate exemption
  - empty approved-caller list
  - multi-symbol caller configuration
- `sc-lint-boundary analyze --format text` and `--format json` both surface
  `SCB-CALLER-001` through the existing analysis command path
- crate README and boundary-enforcement docs both include one canonical
  approved-caller example plus operator guidance on how it differs from
  `references.forbidden`

## Required Validation

- `cargo build --workspace`
- `cargo test -p sc-lint-boundary`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
