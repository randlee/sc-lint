# ADR-009 — Observability Boundary Policy

| Field | Value |
|---|---|
| ID | ADR-009 |
| Status | Accepted |
| Date | 2026-05-24 |
| Deciders | team-lead, clint |
| Relates to | ADR-008, REQ-LOG-001 through REQ-LOG-005 |

## Context

ADR-008 accepted `sc-observability` as the structured logging runtime and
locked in the `CLI-LAYER-OWNS-LOGGER-INITIALIZATION` invariant. That settled
runtime selection, but it did not fully describe which current observability
entry points are part of the approved boundary surface, which validated
command/service metadata types may cross crate boundaries, or how future
direct-linked backend integrations must stay compatible with the CLI-owned
logging model.

By the end of Phase A, the `sc-lint` package contains both:

- library-owned command/context/contract types in `crates/sc-lint/src/`
- binary-only observability glue in `crates/sc-lint/src/main.rs` and
  `crates/sc-lint/src/logging.rs`

The repo needs one accepted policy that explains how those pieces fit together
without reopening the ADR-008 logger-ownership decision.

## Decision

ADR-009 accepts the current observability boundary policy for release `0.1.x`.

### Preserved ADR-008 invariant

`CLI-LAYER-OWNS-LOGGER-INITIALIZATION` remains unchanged:

- only the top-level `sc-lint` binary initializes `sc-observability`
- backend crates must not construct or install logger runtime state
- library code must remain callable without requiring logging to be initialized

ADR-009 clarifies boundary policy around that invariant; it does not weaken or
replace it.

### Approved observability entry points

The current validated observability entry points are:

- `logging::ObservedCommand`
  - binary-only observation context assembled from CLI-owned metadata
- `logging::dispatch_event`
  - the single approved low-level event emission helper
- `contract::ServiceName`
  - the CLI-owned service-identity seam carried in library code without
    importing `sc_observability::ServiceName`
- `CommandEnvelope<T>.command: String`
  - the stable dotted command identifier shared by machine output and logging

These are the approved release-1 observability boundary seams. No new
`emit_*` wrappers, successor context types, or alternate command-identity
fields may be introduced unless a later ADR explicitly reconciles them against
ADR-008 and ADR-009.

### Allowed cross-crate and cross-module type crossings

The following type crossings are permitted by policy:

- library code may expose and pass `CommandContext`, `LoadedConfig`,
  `DispatchTelemetry`, and `CommandEnvelope<T>` because they are CLI-owned
  product contract types rather than `sc-observability` runtime types
- `contract::ServiceName` may cross internal library seams as the validated
  service-identity newtype for later binary conversion
- `CommandEnvelope<T>.command` remains a `String` so the command identity can
  be reused directly by machine output, logging metadata, and delegated backend
  normalization without introducing a second logging-only identifier type

The following crossings are forbidden:

- backend crates taking `Logger`, `LoggerBuilder`, `ActionName`,
  `OutcomeLabel`, `TargetCategory`, or any other `sc-observability` type in
  their public APIs
- backend crates constructing their own observability adapter/context types
  that bypass `CommandContext`, `LoadedConfig`, or the binary-owned
  `ObservedCommand`
- replacing `CommandEnvelope.command` with a logging-only successor field while
  leaving machine output on a different identity contract

### Dependency and ownership policy

For release `0.1.x`:

- the mixed lib+bin `sc-lint` package may keep `sc-observability` in
  `[dependencies]` because Cargo cannot scope it to the binary target alone
- that dependency seam is CLI-only by policy, even though Cargo resolves it at
  the package level
- backend crates remain forbidden from taking direct `sc-observability`
  dependencies:
  - `sc-lint-boundary -> sc-observability`
  - `sc-lint-portability -> sc-observability`
  - `sc-lint-runtime -> sc-observability`
  - `sc-lint-tokio -> sc-observability`

### Constraints on future direct-linked backends

If a later sprint promotes a delegated backend into a direct-linked library
integration:

- the backend may receive CLI-owned context or contract data only after the CLI
  has resolved command identity, service name, config, and output mode
- the backend must not accept `sc-observability` runtime handles in its public
  API as a shortcut for logging ownership
- any backend-originated logging must flow through CLI-owned binary helpers
  using the existing observability entry points rather than backend-local
  logger setup
- any proposal to replace `ObservedCommand`, `dispatch_event`,
  `contract::ServiceName`, or `CommandEnvelope.command` requires explicit ADR
  review because those are now accepted boundary-policy seams

## Consequences

### Positive

- the approved observability surface is explicit instead of implied
- ADR-008 and the boundary records now point at the same current ownership
  model
- future direct-linked backend work has clear guardrails before any runtime
  code changes are proposed

### Negative

- the CLI package still carries `sc-observability` as a package dependency even
  though only the binary module tree may use it directly
- the accepted release-1 seam is intentionally narrow, so future observability
  refactors must pass ADR review rather than landing as local convenience
  changes

## Follow-Up

| Action | Owner | Gate |
|---|---|---|
| Keep `docs/sc-lint/logging.md`, `docs/architecture.md`, and machine-readable boundary/planning records aligned with this accepted seam list. | sc-lint implementation owner | Ongoing through Phase B |
| Reject backend-local `sc-observability` dependencies in boundary review and arch-qa. | repo owner | Ongoing |
| Require explicit ADR reconciliation before introducing new observability context wrappers or replacing `CommandEnvelope.command`. | repo owner | Before any observability runtime redesign |
