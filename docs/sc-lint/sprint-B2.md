---
id: B.2
title: Named Caller Allowlist Enforcement
status: planned
branch: feature/phase-B-sprint-plans
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/phase-B-sprint-plans
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

## Required Work

- extend `BoundaryRecord` inventory parsing with a new callers section instead
  of storing caller policy in prose, comments, or freeform side files
- introduce a typed inventory model for approved-caller entries and validate it
  during TOML load, before graph analysis begins
- add `RuleId::ScbCaller001` and integrate the rule into the existing boundary
  analysis path rather than a one-off analyzer mode
- build caller enumeration on the existing `references_expr` and
  `references_type` edges so the rule shares the already-proven graph source of
  truth
- resolve the caller identity comparison at owner/item granularity so the rule
  remains stable across rendering order and duplicate reference edges
- treat `references.scope` as the only exemption mechanism for owner-crate
  callers; do not add separate suppression flags or warning-only planning modes
- add regression fixtures for:
  - allowed caller
  - disallowed external caller
  - `outside_owner_crate` exemption
  - text output
  - JSON output
- update operator docs to show where the `[callers]` section lives and how it
  differs from `references.forbidden`

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
pub(crate) struct CallersSection {
    pub(crate) approved: Vec<ApprovedCallerEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApprovedCallerEntry {
    pub(crate) symbol: String,
    pub(crate) callers: Vec<String>,
}
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

- `docs/sc-lint/sprint-B2.md` remains the authoritative plan for the sprint and
  states the production-ready expectation for every listed deliverable
- `BoundaryRecord` accepts a `[callers]` section with an `approved` array, and
  malformed rows fail during inventory load
- `SCB-CALLER-001` fires when an external caller reaches an approved-list
  symbol without appearing in that symbol's caller list
- `references.scope = "outside_owner_crate"` exempts owner-crate callers for
  the new rule without exempting external callers
- text and JSON output both report `SCB-CALLER-001` findings through the
  existing `sc-lint-boundary analyze` surface
- crate README and boundary-enforcement docs both include one canonical
  approved-caller example and its enforcement expectations

## Required Validation

- `cargo build --workspace`
- `cargo test -p sc-lint-boundary`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `just lint`
