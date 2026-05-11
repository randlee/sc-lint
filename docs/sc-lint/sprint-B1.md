# Sprint B.1

## Goal

Plan and stage the systemic carry-forward work from the Phase `A` post-mortem
so recurring QA and architecture findings become explicit product and process
deliverables instead of ad hoc cleanup.

## Governing References

- [docs/sc-lint/phase-B-plan.md](./phase-B-plan.md)
- [docs/sc-lint/adr/ADR-008-sc-observability-logging.md](./adr/ADR-008-sc-observability-logging.md)
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)

## Deliverables

1. Post-mortem lint-gate planning for the recurring Phase-A finding families:
   - `RULE-010`: raw identity string literals without named constants
   - `RULE-006`: `/tmp/` paths without intent comments
   - public API error types exposing `anyhow::Error`
   - duplicated `CrateId` newtypes across workspace crates
   - `clippy::for_kv_map` and similar structural for-loop anti-patterns
   - `pub` visibility exceeding the documented contract surface
   - raw `String` fields used for structured identifiers such as
     `boundary_id`, sprint ids, owner ids, and planning keys
2. ADR draft stub
   ([`ADR-009 — Observability Boundary Policy`](./adr/ADR-009-observability-boundary-policy.md))
   for observability boundary policy beyond the current structured-logging
   rollout.
3. QA-process carry-forward: require `rust-best-practices` in
   `practice_mode:all` on every sprint.

## Planned Work

### Lint-Gate Expansion

Document and schedule the next lint-gate additions so the repo can detect the
most common systemic regressions before they escape to late QA:

- identity-literal gate for actor/service names and similar stable identifiers
- intent-comment gate for `/tmp/` fixture paths
- API-error gate for `anyhow::Error` exposure in public types
- shared-newtype gate for canonical workspace identifiers such as `CrateId`
- structural clippy-pattern gate for `for_kv_map`-style issues
- visibility-contract gate for `pub` vs `pub(crate)` drift
- structured-identifier gate for typed `boundary_id`/owner/sprint/planning
  fields

The planned gate contracts for this sprint are:

- API-error gate
  - ban public API error types from exposing `anyhow::Error` or `AnyhowError`
    directly in public signatures, fields, or enum variants
  - require the Phase-A replacement contract already established by
    `RuntimeError`, `BoundaryError`, and `PortabilityError`:
    use an opaque wrapper type where the error surface is crate-owned, or
    `Box<dyn std::error::Error + Send + Sync>` at the public boundary when a
    dynamic source must cross that boundary
  - treat the Phase-A wrapper pattern as the canonical precedent so
    implementers do not invent per-crate ad hoc replacements
  - sprint success criteria:
    - no public API error type exposes `anyhow` in its public signature
    - all public error sources are concrete types or boxed trait objects
- shared-newtype gate
  - declare `sc-lint-schema` as the canonical owner of workspace-shared
    identifier types such as `CrateId`
  - sprint work must consolidate duplicate `CrateId` definitions into that
    canonical crate instead of tolerating parallel local copies
  - sprint success criteria:
    - all shared `CrateId` usage resolves to the canonical definition in
      `sc-lint-schema`
- structured-identifier gate
  - treat raw `String` fields for structured identifiers such as
    `boundary_id`, sprint ids, owner ids, and planning keys as a required-fix
    design issue rather than something to suppress
  - sprint work must introduce the missing identifier newtypes instead of
    adding waivers or documentation-only exceptions
  - sprint success criteria:
    - structured identifier fields use the newtype surface introduced by the
      sprint rather than raw `String`

### Observability ADR

Create a new ADR stub that captures the longer-lived observability boundary
policy for `sc-lint`, including how observability-owned types, entry points,
and future direct-linked backends are allowed to interact across crate
boundaries.

### QA Process

Record the new standing QA expectation:

- every sprint uses `rust-best-practices` in `practice_mode:all`
- the repo should not treat full RBP review as a one-time first-pass activity
  when systemic design drift is still possible

## Acceptance Criteria

1. All seven recurring Phase-A finding patterns are named in the sprint plan.
2. The observability-boundary ADR work is explicitly tracked as a deliverable.
3. The QA-process change calls for `rust-best-practices` in
   `practice_mode:all` on every sprint.
4. The sprint is linked from the Phase-B plan and the project planning index.
