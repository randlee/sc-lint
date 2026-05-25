# `sc-lint` Phase C Plan

This document is the planning stub for Phase `C`, the interface-versioning,
published-interface documentation, and queued shared portability follow-on
phase.

## Objective

Phase `C` introduces a planned `sc-lint-version` capability that turns stable
interface monitoring into one explicit product feature, and it schedules the
next four shared portability lint families that were intentionally left as
follow-on product work after `ADR-010`. The phase must produce both
machine-readable change detection and human-friendly published interface
documentation from the same structured artifacts while also closing the next
consumer-neutral cross-platform portability gaps in `sc-lint-portability`.

## Current Scope

The currently planned sprints in this phase are:

- `C.1`
  - `sc-lint-version` architecture and policy foundation
  - `cargo-semver-checks` selection and Rust public API baseline model
  - dedicated workspace-crate form-factor, `sc-lint check interfaces`
    invocation path, and `[version.families.<family>]` configuration surface
  - explicit breaking-change requirements for Rust APIs, CLI, and
    RPC/socket interfaces
  - see [docs/sc-lint/sprint-C1.md](./sprint-C1.md)
- `C.2`
  - generated published interface artifact pipeline
  - HTML/XHTML/JSON report package model for public APIs and stable contracts
  - versioned CLI baseline artifact schema and generation/update workflow
  - no hand-written monolithic HTML surfaces
  - see [docs/sc-lint/sprint-C2.md](./sprint-C2.md)
- `C.3`
  - hard-fail version gate planning
  - multi-interface-family verdict model, semver-ingestion contract, and
    CI/developer workflow wiring
  - see [docs/sc-lint/sprint-C3.md](./sprint-C3.md)
- `C.4`
  - consumer integration documentation and Claude Code skill planning
  - explicit “what a consuming repo must do” scope
  - see [docs/sc-lint/sprint-C4.md](./sprint-C4.md)
- `C.5`
  - minimal repo-local Claude Code marketplace planning for the adoption skill
  - explicit forwarding/reference path for marketplace advertisement
  - see [docs/sc-lint/sprint-C5.md](./sprint-C5.md)
- `C.6`
  - production path-literal portability parity in `sc-lint-portability`
  - Unix-only and Windows-only absolute path literals in production code
  - see [docs/sc-lint/sprint-C6.md](./sprint-C6.md)
- `C.7`
  - broad production environment-variable portability in
    `sc-lint-portability`
  - `HOME`, `USER`, and `XDG_*` portability checks with platform-neutral
    remediation guidance
  - see [docs/sc-lint/sprint-C7.md](./sprint-C7.md)
- `C.8`
  - shared shell invocation portability in `sc-lint-portability`
  - `Command::new("sh" | "bash")` and hardcoded `/bin/sh` or `/bin/bash`
    production checks
  - see [docs/sc-lint/sprint-C8.md](./sprint-C8.md)
- `C.9`
  - structural cross-platform `cfg` parity enforcement in
    `sc-lint-portability`
  - production `#[cfg(unix)]` branches that lack Windows companions or
    explicit portable fallbacks
  - see [docs/sc-lint/sprint-C9.md](./sprint-C9.md)

## Phase Structure

1. `C.1`
   - decide what is being versioned
   - decide what counts as a breaking change
   - lock the initial Rust public API engine, command surface, and
     family-selection configuration strategy
2. `C.2`
   - define how stable interfaces are published for humans and tooling
   - lock the structured artifact schema, XHTML fragment/report pattern, and
     CLI baseline artifact workflow
3. `C.3`
   - define how version checks hard-fail and where those checks run
   - connect the per-family artifact model to repo gates and release review
4. `C.4`
   - define how consuming repos adopt the capability
   - package the adoption guidance as a repo-local skill
5. `C.5`
   - publish the adoption skill through a minimal repo-local marketplace
6. `C.6`
   - extend shared path-literal portability linting into production code
   - add Windows-path parity to the current Unix-path family
7. `C.7`
   - add one broad production env-portability family for Unix-centric
     home/user/config variables
8. `C.8`
   - add one shared shell invocation portability family for Unix-shell
     assumptions in production code
9. `C.9`
   - add one structural `cfg(unix)` companion-parity rule for production code

## Exit Direction

Phase `C` should leave the repo with:

- an approved versioning approach for Rust public APIs based on
  `cargo-semver-checks`
- an accepted form-factor decision for a dedicated `sc-lint-version` crate
  invoked through `sc-lint check interfaces`
- explicit requirements for CLI and RPC/socket breaking-change detection
- a generated artifact model for published interface documentation:
  - main HTML report
  - JSON sidecar
  - separate XHTML section fragments/panels with built-in copy actions
- a plan for published interface coverage across all shipped crates
- a defined configuration surface under `[version.families.<family>]` for
  selecting interface families and baselines
- a plan for hard-fail version checks against canonical interface artifacts in
  local and CI workflows
- a clear consumer-onboarding plan delivered through:
  - one repo-local skill
  - one minimal marketplace publication path
- an implementation-ready Phase `C` sequence for the next shared portability
  families in `sc-lint-portability`:
  - production Unix-only and Windows-only path-literal detection
  - broad production env-portability checks for `HOME`, `USER`, and `XDG_*`
  - shell invocation portability checks for Unix-shell assumptions
  - structural `cfg(unix)` / `cfg(windows)` parity enforcement
