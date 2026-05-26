---
id: C.10
title: sc-observability 1.1.0 Adoption
status: completed
branch: feature/sprint-C10
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/sprint-C10
target: integration/phase-C
---

# Sprint C.10 — sc-observability 1.1.0 Adoption

## Goal

- adopt `sc-observability` `1.1.0` in `sc-lint`
- verify compatibility of the existing CLI-owned logging seam with the new
  logger typestate, deprecated `emit` replacement, and Windows rotation support
- record the `0.2.x` release-line decision to keep retained-log rotation,
  pruning, and background maintenance enabled through logger-owned defaults
- record that retained-log rotation and maintenance are logger-owned behavior
  when enabled through config rather than wrapper-owned cleanup logic

Issue driver:

- GitHub issue `#57` in `sc-lint`

## Hard Dependencies

- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [docs/project-plan.md](../project-plan.md)
- [docs/sc-lint/logging.md](../sc-lint/logging.md)
- [docs/sc-lint/adr/ADR-008-sc-observability-logging.md](../sc-lint/adr/ADR-008-sc-observability-logging.md)
- [docs/sc-lint/adr/ADR-009-observability-boundary-policy.md](../sc-lint/adr/ADR-009-observability-boundary-policy.md)

## Exact Targets

- `Cargo.toml`
- `crates/sc-lint/Cargo.toml`
- `crates/sc-lint/src/logging.rs`
- `crates/sc-lint/src/main.rs`
- `docs/sc-lint/logging.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/project-plan.md`

## Deliverables

- workspace dependency uplift from `sc-observability` `1.0.0` to `1.1.0`
- explicit compatibility verification for the current logger call sites:
  - `initialize_logger(...)`
  - event log paths that continue to use the supported `emit(...)` API
  - `shutdown(logger)` at the CLI boundary
- one explicit release-line decision for retained-log behavior:
  - enable `RetainedLogPolicy` with documented defaults
  - keep rotation, pruning, and background maintenance logger-owned
- one explicit decision about the new `sc-observe` facade:
  - keep direct `sc-observability` usage because logger construction, file
    sinks, retained-log policy, and health/reporting still require the full
    crate
- Windows rotation compatibility called out as a validation target for the
  release line because `sc-lint` ships on Windows through `xwin`-validated
  paths and Homebrew/GitHub release installs

## Explicit Code Samples

```rust
pub(crate) fn shutdown(logger: Logger) {
    let _ = logger.shutdown();
}
```

```rust
let mut config = LoggerConfig::default_for(service_name, log_root);
config.enable_console_sink = loaded_config.logging_console();
// If retained-log maintenance is enabled for this release line, the nested
// retained-log policy is configured at this seam and the logger owns rotation,
// pruning, and maintenance behavior.
```

```rust
// C.10 stays on the supported sc-observability 1.1.0 public API:
logger.emit(event)?;
```

## This Sprint Does Not Close

- a redesign of the CLI-owned observability boundary
- backend-crate direct `sc-observability` dependencies
- OTLP or remote observability rollout

## Acceptance Criteria

- the sprint identifies the exact `sc-lint` logger construction and shutdown
  seams that must compile unchanged or be minimally adapted for
  `Logger<Running>` / `Logger<Stopped>`
- the sprint identifies the exact event call sites and confirms that
  `sc-observability` `1.1.0` keeps `emit(...)` as the supported public API for
  `Logger<Running>`
- the plan keeps the CLI-only `sc-observability` dependency seam from
  `ADR-009`
- the sprint makes one explicit yes/no decision on retained-log policy for the
  `0.2.x` line instead of leaving rotation/pruning/maintenance implicit
- when retained logging is enabled, the sprint states that the logger itself
  owns rotation/pruning/background maintenance based on configured settings
- the sprint makes one explicit no decision on adopting `sc-observe` in
  `0.2.x`
- the sprint records that `shutdown(logger)` takes ownership by value because
  the logger transitions from `Logger<Running>` to `Logger<Stopped>` on
  shutdown
- `docs/sc-lint/logging.md`, `docs/requirements.md`, and `docs/architecture.md`
  remain aligned on the chosen `1.1.0` integration shape

## Required Validation

- `just lint`
