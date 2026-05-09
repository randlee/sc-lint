# ADR-006 — AI-First Top-Level CLI Contract

| Field | Value |
|---|---|
| ID | ADR-006 |
| Status | **Accepted** |
| Date | 2026-05-09 |
| Deciders | team-lead, quality-mgr, clint |
| Relates to | REQ-PRODUCT-001 through REQ-PRODUCT-006F, REQ-CLI-001 through REQ-CLI-008E |

---

## Context

`sc-lint` is planning a top-level CLI that coordinates multiple backend tools,
some in Rust and some in Python during migration windows.

Without an explicit contract decision, the CLI could drift into one of two weak
states:

- a human-first wrapper that only incidentally supports machine use
- a thin dispatcher whose backend-specific machine contracts leak through to
  users

The `creating-ai-clis` and `reviewing-ai-clis` skill guidance sets a stricter
target:

- machine-readable output is primary
- top-level failures stay machine-readable when machine mode is requested
- request/response seams remain reusable outside the CLI entrypoint
- human-readable output is secondary

## Decision Drivers

- `sc-lint` is intended for direct automation and agent use
- backend implementations will evolve over time, but the user-facing machine
  contract should remain stable
- future MCP wrappers should be able to reuse the same business request and
  response models without reshaping
- future interactive graph exploration should not weaken the scriptable CLI
  contract

## Options Considered

1. Treat the top-level CLI as a dispatcher only and allow machine contracts to
   remain backend-specific.
2. Make the top-level CLI the stable AI-first machine contract owner for
   non-interactive commands while still delegating backend execution internally.

## Decision

`sc-lint` adopts option 2.

The top-level CLI is both:

- the orchestration layer for backend tools
- the stable user-facing machine contract owner for non-interactive commands

Key decisions:

- canonical machine mode:
  - `--json`
- machine-readable failures remain required in machine mode
- human-readable output is a presentation layer and must not expose
  machine-significant information unavailable through `--json`
- backend-specific flags such as `--format json` may be used internally during
  migration, but they are not part of the stable top-level contract
- future interactive or TUI graph features remain secondary surfaces and must
  not become the only path to machine-significant data

Important planned contract types:

- `Cli`
- `Command`
- `LintProfile`
  - `Fast`
  - `Full`
  - `Ci`
- `OutputMode`
  - `Human`
  - `Json`
- `CliError`

## Consequences

### Positive

- the CLI has a clear product identity for automation and agent use
- backend migration does not force user-facing contract churn
- future MCP work can reuse the same request/response seams
- interactive graph features can be added without weakening scriptability

### Negative

- the CLI must own more explicit contract design than a thin dispatcher model
- backend-specific output models may need normalization or adaptation
- machine-readable top-level failure behavior must be implemented carefully

## Follow-Up

| Action | Owner | Gate |
|---|---|---|
| Define the top-level `--json` contract, stable error envelope, and contract-type names in the CLI requirements and architecture docs. | sc-lint implementation owner | Before CLI implementation starts |
| Keep backend-specific machine flags behind the top-level CLI contract during migration. | sc-lint implementation owner | Ongoing during extraction |
| Treat future interactive graph exploration as a secondary surface and require machine-readable parity for significant data. | repo owner | Before interactive graph features land |

*ADR-006 | sc-lint | 2026-05-09*
