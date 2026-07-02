# ADR-004 — Structured Boundary Definitions, Package Dependency Policy, And Planning-Aware Inventory Parity

| Field | Value |
|---|---|
| ID | ADR-004 |
| Status | **Accepted** |
| Date | 2026-05-06 |
| Deciders | arch-inj, team-lead, clint |
| Relates to | REQ-SCB-001 through REQ-SCB-021 |

---

## Context

`sc-lint-boundary` now has enough AST and graph machinery to enforce concrete
structural rules, but the current Markdown-embedded boundary-record model is a
poor long-term source for:

- inventory-parity checks such as "documented item exists in code"
- planning-aware warn/error escalation for future-sprint gaps
- extraction of `sc-lint` into its own repository

The tool needs one machine-authoritative source for:

- boundary definitions
- package-level dependency policy
- planning metadata for missing documented items
- deterministic warning-to-error escalation rules

## Decision Drivers

- boundary definitions must be machine-authoritative and easy to validate
- package dependency policy must come from the same canonical boundary source
  rather than from prose-only review guidance
- inventory-parity enforcement must produce deterministic warn/error outcomes
- planned future-sprint gaps must stay visible without turning into indefinite
  suppressions
- the format must remain neutral enough for future `sc-lint` extraction into a
  standalone repository

## Options Considered

1. Keep Markdown-embedded records as the canonical source.
2. Move canonical boundary definitions to standalone TOML and keep Markdown as
   the human-facing explanation layer.

## Decision

`sc-lint` adopts the following model:

- canonical machine-readable boundary definitions live in standalone TOML files
  under `boundaries/`
- package dependency allowlists and forbidden direct workspace edges are also
  modeled in those same TOML boundary records
- planning metadata for inventory-parity enforcement lives in
  `boundaries/planning.toml`
- package dependency policy is enforced separately from manifest
  workspace/version hygiene even when both features consume Cargo metadata
- inventory-parity checks compare structured boundary items against the code
  graph at item-key granularity
- missing documented items may warn only when they have a valid structured
  future-sprint mapping
- unplanned or overdue missing documented items fail as errors

## Consequences

### Positive

- boundary inventories become directly parseable without Markdown fenced-block
  extraction
- package-edge dependency rules can be enforced from the same canonical
  boundary data instead of relying on documentation drift or review memory
- warn/error behavior becomes deterministic rather than prose-driven
- the tool can fail new architectural drift immediately while still surfacing
  planned future work
- future `sc-lint` extraction becomes simpler because the canonical data model
  is already repo-neutral

### Negative

- the dual-loader migration must exist for one transition period
- consumer repositories must maintain a structured `boundaries/planning.toml`
  file once inventory-parity enforcement begins

## Follow-Up

| Action | Owner | Gate |
|---|---|---|
| Keep duplicate-source equivalence mode test-only and disabled in normal lint runs and CI. | sc-lint implementation owner | Before enabling dual-loader support in normal developer lint runs |
| Implement `SCB-DEPENDENCY-001`, `SCB-DEPENDENCY-002`, and `SCB-DEPENDENCY-003` against TOML-backed boundary dependency metadata. | sc-lint implementation owner | Before claiming package-edge dependency enforcement in normal lint runs or release docs |
| Implement `SCB-INVENTORY-001`, `SCB-INVENTORY-002`, and `SCB-INVENTORY-003` against TOML-backed boundary data. | sc-lint implementation owner | Before inventory-parity enforcement enters CI |
| Make `[planning].current_sprint` in `boundaries/planning.toml` the authoritative current-sprint source for warn/error escalation. | consumer repository owner | Before planned-gap warnings are permitted in normal lint runs |

*ADR-004 | sc-lint | 2026-05-06*
