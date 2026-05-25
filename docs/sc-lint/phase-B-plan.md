# `sc-lint` Phase B Plan

This document is the execution stub for Phase `B`, the post-Phase-A hardening
and process-tightening phase.

## Objective

Phase `B` turns the recurring systemic findings from Phase `A` into explicit
planned engineering work so the same defect patterns stop reappearing during
later sprint implementation and QA.

## Current Scope

The currently planned sprints in this phase are:

- `B.1`
  - post-mortem carry-forward lint-gate backlog hardening
  - portability-scope hardening for Windows-path, env, and shell portability
  - portability ownership/parity ADR coverage
  - see [docs/sc-lint/sprint-B1.md](./sprint-B1.md)
- `B.2`
  - TOML-backed named-caller allowlist enforcement in `sc-lint-boundary`
  - new `SCB-CALLER-001` rule using the existing reference graph
  - CLI/config/documentation integration for approved-caller policy
  - see [docs/sc-lint/sprint-B2.md](./sprint-B2.md)
- `B.3`
  - observability boundary-policy ADR acceptance plus doc/boundary alignment
  - promote `ADR-009` from stub to accepted policy
  - see [docs/sc-lint/sprint-B3.md](./sprint-B3.md)
- `B.4`
  - QA-process hardening
  - triage-first routing and QA-1-only rust-best-practices default
  - see [docs/sc-lint/sprint-B4.md](./sprint-B4.md)
- `sprint-B-homebrew`
  - full `sc-lint` Homebrew distribution planning
  - primary `brew install randlee/tap/sc-lint` path
  - release/tap update strategy for the full binary set
  - sprint number intentionally TBD; keep this item at the end of the Phase `B`
    sequence until numbering is assigned after the `B.1`-`B.4`
    implementation order is reviewed and locked
  - see [docs/sc-lint/sprint-B-homebrew.md](./sprint-B-homebrew.md)

## Phase Structure

Phase `B` currently starts with four focused planning-and-hardening sprints,
followed by one queued distribution-planning sprint whose numeric slot is still
open:

1. `B.1`
   - encode Phase-A post-mortem findings as planned product/process work
   - define the next lint gates and architecture-policy follow-ups
   - tighten QA expectations before additional Phase-B feature scope begins
2. `B.2`
   - convert approved-caller policy from prose into TOML-backed enforcement
   - add the next boundary-rule family needed to stop review-only caller drift
3. `B.3`
   - close observability boundary-policy ADR work only
4. `B.4`
   - close QA-process hardening only
5. `sprint-B-homebrew`
   - reserve the final Phase `B` slot for the Homebrew full-toolset rollout
   - assign the final sprint number only after the numbered implementation
     sequence around it is decided

Additional Phase `B` sprint scope may be added after the current numbered
planning line through `B.4` is reviewed.

## Exit Direction

Phase `B` should leave the repo with:

- explicit planned ownership for the recurring Phase-A defect families
- an explicit product-side backlog for reusable consumer-proven lint gaps
  without importing consumer-specific wrapper names or report formats into the
  core tool contract
- an explicit Phase-B portability expansion line covering Windows-only path
  literals, broader env portability checks, and shell-portability linting
- accepted ADR coverage for shared portability ownership/parity and
  observability boundary policy
- a production-ready plan for caller-identity enforcement in
  `sc-lint-boundary`
- a documented QA-process line with triage-first routing and QA-1-only broad
  rust-best-practices review
- a numbered-or-explicitly-queued plan for moving Homebrew from a
  boundary-only stopgap to the full released `sc-lint` toolset
