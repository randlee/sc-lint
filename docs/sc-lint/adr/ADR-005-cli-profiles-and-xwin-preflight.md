# ADR-005 — Top-Level CLI Profiles And Xwin Preflight

| Field | Value |
|---|---|
| ID | ADR-005 |
| Status | **Accepted** |
| Date | 2026-05-08 |
| Deciders | arch-inj, team-lead, clint |
| Relates to | REQ-PRODUCT-001 through REQ-PRODUCT-016A, REQ-CLI-001 through REQ-CLI-015 |

---

## Context

`sc-lint` needs a stable top-level CLI, but the command surface cannot be left
implicit in `Justfile` recipes alone.

At the same time, developers want earlier Windows-drift signal without
pretending cross-target preflight is a substitute for real Windows CI.

The product therefore needs explicit decisions about:

- lint profile naming
- the difference between lint-only CI parity and top-level CI-equivalent
  execution
- how `cargo xwin` participates when installed

## Decision Drivers

- the user-facing CLI contract must be stable and discoverable
- profile membership should be a product decision, not hidden `Justfile`
  behavior
- Windows preflight should help local development without distorting CI parity
- real Windows CI must remain the authoritative validation path

## Options Considered

1. Keep profile semantics in `Justfile` only and expose no first-class CLI
   shape for them.
2. Define first-class CLI profiles and explicit `xwin` command modes, while
   keeping `xwin` out of the CI-parity lint profile.

## Decision

`sc-lint` adopts the following command and profile model:

- lint profiles:
  - `sc-lint lint fast`
  - `sc-lint lint full`
  - `sc-lint lint ci`
- top-level CI-equivalent command:
  - `sc-lint ci`
- explicit `xwin`-aware commands when `cargo xwin` is installed:
  - `sc-lint check xwin`
  - `sc-lint clippy xwin`

Profile policy:

- `fast`
  - local low-latency profile
  - may include `xwin check` when available and fast enough
- `full`
  - stronger local pre-push profile
  - may include `xwin check` and `xwin clippy` when available
- `ci`
  - lint-only CI-parity profile
  - excludes `xwin`
- top-level `ci`
  - runs lint plus tests

## Consequences

### Positive

- the CLI contract becomes clearer than a `Justfile`-only model
- developers get a clear separation between:
  - local fast/full lint behavior
  - lint-only CI parity
  - full CI-equivalent execution
- `xwin` can improve local confidence without complicating real CI semantics

### Negative

- the CLI has to own profile semantics explicitly
- capability detection for `cargo xwin` must be implemented cleanly
- some local results will still differ from real Windows CI because `xwin`
  only covers compile-oriented preflight

## Follow-Up

| Action | Owner | Gate |
|---|---|---|
| Implement the top-level `sc-lint` CLI crate with explicit `fast`, `full`, `ci`, and top-level `ci` command surfaces. | sc-lint implementation owner | Before release `0.1.x` CLI stabilization |
| Add capability detection for `cargo xwin` and wire `xwin check` / `xwin clippy` into the appropriate local profiles. | sc-lint implementation owner | Before any `xwin` profile behavior becomes part of default local workflows |
| Keep real Windows CI as the authoritative validation path and do not add `xwin` to the `ci` lint profile. | repo owner | Ongoing |

*ADR-005 | sc-lint | 2026-05-08*
