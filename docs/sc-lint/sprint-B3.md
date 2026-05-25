---
id: B.3
title: Observability Boundary Policy ADR
status: planned
branch: feature/phase-B-sprint-plans
worktree: <repo-worktree>/feature/phase-B-sprint-plans
target: develop
---

# Sprint B.3 — Observability Boundary Policy ADR

## Goal

- accept the broader observability boundary policy that remains only stubbed in
  `ADR-009`
- align observability ownership language across ADR, architecture, and planning
  docs
- close the remaining design ambiguity around how observability-owned types and
  entry points may cross crate boundaries

## Hard Dependencies

- [docs/sc-lint/adr/ADR-008-sc-observability-logging.md](./adr/ADR-008-sc-observability-logging.md)
- [docs/sc-lint/adr/ADR-009-observability-boundary-policy.md](./adr/ADR-009-observability-boundary-policy.md)
- [docs/sc-lint/logging.md](./logging.md)
- [docs/sc-lint/phase-B-plan.md](./phase-B-plan.md)

## Exact Targets

- `docs/sc-lint/adr/ADR-009-observability-boundary-policy.md`
- `docs/sc-lint/adr/README.md`
- `docs/sc-lint/phase-B-plan.md`
- `docs/project-plan.md`
- `docs/architecture.md`
- `docs/sc-lint/logging.md`
- `docs/sc-lint/README.md`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- `ADR-009` is promoted from draft stub to accepted policy text
- the accepted ADR defines the approved observability entry points for
  `sc-lint`, the allowed cross-crate type crossings, and the constraints on
  future direct-linked backend observability integration
- architecture, logging, and planning docs align with the accepted
  observability boundary policy without weakening the CLI-owned logger
  initialization invariant from `ADR-008`

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```rust
pub struct CommandObservability {
    pub command: CommandId,
    pub service_name: ServiceName,
}

pub fn dispatch_command(
    observability: &CommandObservability,
    command: Command,
) -> Result<CommandEnvelope<RenderedOutput>, CliError>;
```

```rust
pub fn emit_cli_command_started(
    observability: &CommandObservability,
    args: &[String],
);

pub fn emit_cli_command_completed(
    observability: &CommandObservability,
    verdict: CommandVerdict,
    summary: &str,
    elapsed_ms: u64,
);
```

```rust
pub struct CommandEnvelope<T> {
    pub ok: bool,
    pub command: CommandId,
    pub data: Option<T>,
    pub error: Option<CliError>,
}
```

```rust
pub struct CommandId(String);
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
- the sprint doc leaves no ambiguity about which validated command/service
  metadata types may cross crate boundaries and which observability setup work
  remains CLI-owned only
- `docs/architecture.md`, `docs/sc-lint/logging.md`, and the ADR index align
  with the accepted `ADR-009` text

## Required Validation

- `just lint`
