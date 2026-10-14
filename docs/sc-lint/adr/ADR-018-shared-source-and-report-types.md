# ADR-018 — Shared Source Directives And Report Types (Draft)

| Field | Value |
|---|---|
| ID | ADR-018 |
| Status | Draft; requires Rand approval |
| Deciders | Proposed: sc-lint maintainers; approval pending |
| Relates to | ADR-004, ADR-006, ADR-007, REQ-PRODUCT-002DA, REQ-PRODUCT-003 |

## Context

The workspace has separate shared crates for source attributes, parsed
directives, and analyzer report types. Their package boundaries are visible in
the code and top-level architecture, but the crate-level ownership decisions
are not collected in an accepted ADR. This draft records the current code
shape for review; it is not an approved change to product policy.

## Proposed decision

1. `sc-lint-attributes` exposes the compile-valid `#[sc_lint(...)]` proc-macro
   namespace and delegates shared syntax interpretation to
   `sc-lint-directives`.
2. `sc-lint-directives` owns parsed directive types and syntax validation used
   by the proc macro and analyzers. It does not decide rule findings.
3. `sc-lint-schema` owns neutral transport types for findings and analyzer
   reports. Analyzer policy stays in analyzer crates, and the top-level CLI
   normalizes backend payloads into its stable envelope.
4. Shared source and report crates do not depend on analyzer implementations
   or the top-level CLI.

## Consequences

- Attribute syntax has one shared parse representation.
- Source declarations, semantic enforcement, and machine transport remain
  separate ownership surfaces.
- These boundaries become accepted policy only after Rand reviews and accepts
  this ADR.

## Approval

Pending. Do not treat this proposal as an adopted requirement until approved.
