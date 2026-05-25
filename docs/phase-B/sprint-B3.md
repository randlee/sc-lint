---
id: B.3
title: Observability Boundary Policy ADR
status: completed
branch: feature/sprint-B3
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/sprint-B3
target: integration/phase-B
---

# Sprint B.3 — Observability Boundary Policy ADR

## Goal

- accept the broader observability boundary policy that remains only stubbed in
  `ADR-009`
- align observability ownership language across ADR, architecture,
  machine-readable boundary records, and planning docs
- close the remaining design ambiguity around how observability-owned types and
  entry points may cross crate boundaries

## Hard Dependencies

- [docs/sc-lint/adr/ADR-008-sc-observability-logging.md](../sc-lint/adr/ADR-008-sc-observability-logging.md)
- [docs/sc-lint/adr/ADR-009-observability-boundary-policy.md](../sc-lint/adr/ADR-009-observability-boundary-policy.md)
- [docs/sc-lint/logging.md](../sc-lint/logging.md)
- [docs/phase-B/phase-B-plan.md](./phase-B-plan.md)

## Exact Targets

- `docs/sc-lint/adr/ADR-009-observability-boundary-policy.md`
- `docs/sc-lint/adr/README.md`
- `docs/phase-B/phase-B-plan.md`
- `docs/project-plan.md`
- `docs/architecture.md`
- `docs/sc-lint/logging.md`
- `docs/sc-lint/README.md`
- `boundaries/sc-lint/top-level-cli.toml`
- `boundaries/planning.toml`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `ADR-009` is promoted from draft stub to accepted policy text
- the accepted ADR defines the approved observability entry points for
  `sc-lint`, the allowed cross-crate type crossings, and the constraints on
  future direct-linked backend observability integration
- the accepted ADR explicitly maps the allowed logging contract surface back to
  ADR-008, keeping `logging::ObservedCommand`, `logging::dispatch_event`, the
  CLI-owned `contract::ServiceName` seam, and the existing `CommandEnvelope`
  `command: String` field unless ADR-009 records a reconciled successor
- architecture, logging, and planning docs align with the accepted
  observability boundary policy without weakening the CLI-owned logger
  initialization invariant from `ADR-008`
- the machine-readable boundary and planning records that describe CLI-owned
  observability dependency seams align with the accepted policy, so backend
  crates remain forbidden from taking direct `sc-observability` dependencies

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
pub(crate) struct ObservedCommand<'a> {
    context: &'a CommandContext,
    loaded_config: &'a LoadedConfig,
}

fn dispatch_event(
    logger: &Logger,
    observed: &ObservedCommand<'_>,
    outcome: OutcomeLabel,
    action: ActionName,
    fields: Map<String, Value>,
);
```

```rust
pub(crate) struct ServiceName(&'static str);

impl ServiceName {
    pub(crate) const fn new(value: &'static str) -> Self;
    pub(crate) const fn as_str(&self) -> &str;
}
```

```rust
pub struct CommandEnvelope<T> {
    pub ok: bool,
    pub command: String,
    pub data: Option<T>,
    pub error: Option<CliError>,
}
```

```toml
[dependencies]
allowed_dependencies = ["sc-lint-boundary", "sc-lint-schema", "sc-observability"]
forbidden_edges = [
  "sc-lint-attributes -> sc-lint",
  "sc-lint-boundary -> sc-observability",
  "sc-lint-portability -> sc-observability",
  "sc-lint-runtime -> sc-observability",
  "sc-lint-tokio -> sc-observability",
]
```

## This Sprint Does Not Close

- implementation of new observability backends
- runtime changes outside the documented observability ownership policy
- QA-process prompt changes

## Acceptance Criteria

- `ADR-009` status is `Accepted`
- `ADR-009` explicitly defines:
  - approved observability entry points
  - permitted observability-owned type crossings
  - constraints for future direct-linked backend observability integration
  - the preserved `CLI-LAYER-OWNS-LOGGER-INITIALIZATION` invariant from
    `ADR-008`
- the sprint doc leaves no ambiguity about which current validated command and
  service metadata types may cross crate boundaries and which observability
  setup work remains CLI-owned only
- the sprint doc does not introduce forbidden `emit_*` wrappers or undefined
  successor types without explicit ADR-008 reconciliation in ADR-009
- `docs/architecture.md`, `docs/sc-lint/logging.md`, the ADR index, and the
  relevant machine-readable boundary/planning records align with the accepted
  `ADR-009` text
- `boundaries/sc-lint/top-level-cli.toml` and the observability-related
  planning records preserve the CLI-only `sc-observability` dependency seam
  and the backend `forbidden_edges` documented by the accepted policy

## Required Validation

- `cargo build --workspace`
- `just lint sc-boundary`
- `just lint`
