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
   `/Users/randlee/.claude/skills/html-report/SKILL.md`.
4. The generated JSON sidecar is the canonical machine-readable baseline;
   main HTML reports and optional XHTML fragments are human-facing derivatives
   of that structured source.
5. The versioning model applies to multiple interface families:
   - Rust public APIs for all shipped crates
   - stable top-level CLI commands and machine contracts
   - RPC/socket interfaces when such surfaces exist

## Consequences

- `sc-lint-version` planning must define separate breaking-change semantics for
  Rust APIs, CLI contracts, and RPC/socket interfaces
- the repo must inventory and publish more than just Rust library APIs
- generated report templates become part of the versioning pipeline contract
- hand-authored HTML monoliths are explicitly out of scope for canonical
  published artifacts
- future implementation must carry both machine-readability and human
  readability from one shared structured artifact model

## Follow-on Planning

- define the requirements for what counts as a breaking change per interface
  family
- define the artifact schema for published interface report packages
- define the sprint sequence for:
  - Rust public API baseline integration
  - generated interface report publication
  - hard-fail policy enforcement across all supported interface families
