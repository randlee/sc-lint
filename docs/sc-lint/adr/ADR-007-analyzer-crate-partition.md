# ADR-007 — Analyzer-Crate Partition And Primary Lint Target Mapping

| Field | Value |
|---|---|
| ID | ADR-007 |
| Status | **Accepted** |
| Date | 2026-05-09 |
| Deciders | team-lead, quality-mgr, clint |
| Relates to | REQ-PRODUCT-003 through REQ-PRODUCT-006F, REQ-PRODUCT-015A through REQ-PRODUCT-015C, REQ-CLI-007F through REQ-CLI-007G |

---

## Context

`sc-lint` is no longer a two-crate spike. The product is now planning multiple
specialized analyzer crates and a top-level CLI that groups them into one
tool family.

Without an explicit partition decision, two kinds of drift are likely:

- new rule families get appended to whichever crate already exists
- the top-level CLI exposes convenience names that hide crate ownership and
  make the backend boundary model ambiguous

The recent portability and runtime imports from `atm-core` make that risk
immediate.

## Decision Drivers

- backend crates should remain narrowly scoped and independently evolvable
- rule-family ownership should be obvious from the product command surface
- release `0.1.x` should avoid growing `sc-lint-boundary` into a catch-all
  analyzer
- Tokio-specific semantics should not be mixed into the generic std runtime
  crate by default

## Options Considered

1. Keep new rule families in `sc-lint-boundary` and expose subset-oriented
   top-level command names only.
2. Partition analyzer families into dedicated crates and make the primary CLI
   lint targets track crate ownership directly.

## Decision

`sc-lint` adopts option 2.

Current and planned analyzer partition:

- `sc-lint-boundary`
  - boundary inventory, ownership, and attribute-driven boundary policy
- `sc-lint-portability`
  - shared OS/platform portability rules
- `sc-lint-runtime`
  - shared std runtime/concurrency correctness rules
- `sc-lint-tokio`
  - reserved future crate for Tokio-specific runtime rules only

Primary CLI lint-target mapping:

- `sc-lint lint sc-boundary`
- `sc-lint lint sc-portability`
- `sc-lint lint sc-runtime`
- future:
  - `sc-lint lint sc-tokio`

Grouped aliases such as `unix-gating` or `runtime-waits` may exist later, but
they are secondary convenience surfaces. They do not replace the primary
crate-mapped command surface.

## Consequences

### Positive

- rule-family ownership stays explicit
- backend crates can remain small and modular
- the top-level CLI exposes a predictable mapping from command target to
  analyzer ownership
- future Tokio-specific work has a reserved home without overloading the std
  runtime crate

### Negative

- more crate and boundary records must be maintained in planning metadata
- the top-level CLI must document both primary crate-mapped targets and any
  secondary grouped aliases
- some backend integrations will need an explicit choice between direct
  dependency and delegated subprocess execution

## Follow-Up

| Action | Owner | Gate |
|---|---|---|
| Create structured boundary/planning records for planned analyzer crates before their implementation sprints start. | sc-lint implementation owner | Before A.4 and A.5 execution |
| Keep primary lint targets crate-mapped in CLI requirements and architecture docs. | sc-lint implementation owner | Before top-level CLI implementation starts |
| Keep Tokio-specific rules out of `sc-lint-runtime` unless a later ADR changes that split. | repo owner | Ongoing |

*ADR-007 | sc-lint | 2026-05-09*
