# `sc-lint` Phase C Plan

This document is the planning stub for Phase `C`, the interface-versioning and
published-interface documentation phase.

## Objective

Phase `C` introduces a planned `sc-lint-version` capability that turns stable
interface monitoring into one explicit product feature. The phase must produce
both machine-readable change detection and human-friendly published interface
documentation from the same structured artifacts.

## Current Scope

The currently planned sprints in this phase are:

- `C.1`
  - `sc-lint-version` architecture and policy foundation
  - `cargo-semver-checks` selection and Rust public API baseline model
  - explicit breaking-change requirements for Rust APIs, CLI, and
    RPC/socket interfaces
  - see [docs/sc-lint/sprint-C1.md](./sprint-C1.md)
- `C.2`
  - generated published interface artifact pipeline
  - HTML/XHTML/JSON report package model for public APIs and stable contracts
  - no hand-written monolithic HTML surfaces
  - see [docs/sc-lint/sprint-C2.md](./sprint-C2.md)
- `C.3`
  - hard-fail version gate planning
  - multi-interface-family verdict model and CI/developer workflow wiring
  - see [docs/sc-lint/sprint-C3.md](./sprint-C3.md)
- `C.4`
  - consumer integration documentation and Claude Code skill planning
  - explicit “what a consuming repo must do” scope
  - see [docs/sc-lint/sprint-C4.md](./sprint-C4.md)
- `C.5`
  - minimal repo-local Claude Code marketplace planning for the adoption skill
  - explicit forwarding/reference path for marketplace advertisement
  - see [docs/sc-lint/sprint-C5.md](./sprint-C5.md)

## Phase Structure

1. `C.1`
   - decide what is being versioned
   - decide what counts as a breaking change
   - lock the initial Rust public API engine and baseline strategy
2. `C.2`
   - define how stable interfaces are published for humans and tooling
   - lock the structured artifact schema and XHTML fragment/report pattern
3. `C.3`
   - define how version checks hard-fail and where those checks run
   - connect the per-family artifact model to repo gates and release review
4. `C.4`
   - define how consuming repos adopt the capability
   - package the adoption guidance as a repo-local skill
5. `C.5`
   - publish the adoption skill through a minimal repo-local marketplace

## Exit Direction

Phase `C` should leave the repo with:

- an approved versioning approach for Rust public APIs based on
  `cargo-semver-checks`
- explicit requirements for CLI and RPC/socket breaking-change detection
- a generated artifact model for published interface documentation:
  - main HTML report
  - JSON sidecar
  - separate XHTML section fragments/panels with built-in copy actions
- a plan for published interface coverage across all shipped crates
- a plan for hard-fail version checks against canonical interface artifacts in
  local and CI workflows
- a clear consumer-onboarding plan delivered through:
  - one repo-local skill
  - one minimal marketplace publication path
