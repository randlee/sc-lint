# ADR-011 — Interface Versioning And Published Artifacts

| Field | Value |
|---|---|
| ID | ADR-011 |
| Status | Draft |
| Date | 2026-05-25 |
| Deciders | team-lead, clint |

## Context

`sc-lint` currently has multiple stable or intended-stable interface surfaces:

- Rust crate public APIs
- the top-level CLI command and machine-contract surface
- future RPC/socket interfaces if they are introduced

The repo does not yet have one unified mechanism that both:

1. publishes human-friendly interface documentation
2. blocks breaking interface changes from landing unnoticed

The planning direction approved for this line is:

- start Rust public API checking with `cargo-semver-checks`
- publish generated HTML reports plus machine-readable sidecars
- use those canonical interface artifacts as the basis for hard-fail checks
- avoid hand-written monolithic HTML documents

## Decision

1. `sc-lint` will plan a dedicated `sc-lint-version` capability as the owner
   of interface-version checking and published interface artifacts.
2. The initial Rust public API decision engine will be
   `cargo-semver-checks` rather than a custom semver implementation.
3. Published interface documentation will be generated from structured input
   and reusable templates using the XHTML-fragment/report pattern defined by
   [docs/sc-lint/interface-reporting-constraints.md](../interface-reporting-constraints.md).
4. The generated JSON sidecar is the canonical machine-readable baseline;
   main HTML reports and separate XHTML section fragments/panels are
   human-facing derivatives of that structured source.
5. The reusable HTML/XHTML template stack must expose built-in copy actions
   per section/panel for canonical JSON payload and canonical context text
   rather than leaving copy behavior to per-report custom scripting.
6. The versioning model applies to multiple interface families:
   - Rust public APIs for all shipped crates
   - stable top-level CLI commands and machine contracts
   - RPC/socket interfaces when such surfaces exist
7. Consumer adoption guidance for `sc-lint-version` will be planned as a
   first-class product artifact, delivered through a repo-local Claude Code
   skill.
8. Marketplace advertisement for that adoption skill will be planned as a
   separate minimal repo-local marketplace closure rather than bundled into
   the skill-design sprint.

## Consequences

- `sc-lint-version` planning must define separate breaking-change semantics for
  Rust APIs, CLI contracts, and RPC/socket interfaces
- the repo must inventory and publish more than just Rust library APIs
- generated report templates, including built-in panel copy controls, become
  part of the versioning pipeline contract
- hand-authored HTML monoliths are explicitly out of scope for canonical
  published artifacts
- future implementation must carry both machine-readability and human
  readability from one shared structured artifact model
- consumer repos will need a documented adoption path, and the planning line
  must keep skill design and marketplace publication as separate closures
  rather than prose hidden in scattered docs

## Follow-on Planning

- define the requirements for what counts as a breaking change per interface
  family
- define the artifact schema for published interface report packages
- define the sprint sequence for:
  - Rust public API baseline integration
  - generated interface report publication
  - hard-fail policy enforcement across all supported interface families
  - consumer-adoption skill design
  - minimal marketplace publication for that adoption skill
