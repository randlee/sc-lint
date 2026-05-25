# ADR-009 — Observability Boundary Policy

| Field | Value |
|---|---|
| ID | ADR-009 |
| Status | Draft |
| Date | 2026-05-11 |
| Deciders | team-lead, clint |

## Purpose

This ADR stub is reserved for the broader observability boundary policy work
planned in Sprint `B.3`.

## Planned Scope

- define the approved observability entry points for `sc-lint`
- define how observability-owned types are permitted to cross crate
  boundaries
- document the policy for future direct-linked backend observability
  integration
- align boundary records and ADR language for observability-specific
  ownership/invariants

## Constraints

- This ADR is subordinate to ADR-008
  ([`ADR-008-sc-observability-logging.md`](./ADR-008-sc-observability-logging.md)).
- The `CLI-LAYER-OWNS-LOGGER-INITIALIZATION` invariant defined in ADR-008
  must be preserved.
- ADR-009 policy decisions must not override or weaken the CLI-owned logger
  initialization model accepted in ADR-008.

## Status Note

This ADR is intentionally a draft stub during Phase-B planning. The accepted
policy text is expected to be written as part of Sprint `B.3`.
