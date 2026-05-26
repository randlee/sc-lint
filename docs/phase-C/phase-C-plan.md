# `sc-lint` Phase C Plan

This document is the planning stub for Phase `C`, the interface-versioning,
shared reporting, observability-maintenance, and queued shared portability
follow-on phase.

## Objective

Phase `C` introduces a planned `sc-lint-version` capability that turns stable
interface monitoring into one explicit product feature, and it schedules the
next four shared portability lint families that were intentionally left as
follow-on product work after `ADR-010`. The phase must produce both
machine-readable change detection and a shared human-facing report model from
the same structured artifacts, queue one explicit `sc-observability` `1.1.0`
adoption pass, and close the next consumer-neutral cross-platform portability
gaps in `sc-lint-portability`.

## Current Scope

The currently planned sprints in this phase are:

- `C.1`
  - `sc-lint-version` architecture and policy foundation
  - `cargo-semver-checks` selection and Rust public API baseline model
  - dedicated workspace-crate form-factor, `sc-lint check interfaces`
    invocation path, and `[version.families.<family>]` configuration surface
  - explicit breaking-change requirements for Rust APIs, CLI, and
    RPC/socket interfaces
  - see [docs/phase-C/sprint-C1.md](./sprint-C1.md)
- `C.2`
  - shared report-template pipeline planning
  - `sc-compose`/Jinja report package model for public API, CLI, and ICD
    reports
  - template-selection and override contract for consumers
  - no hand-written monolithic HTML surfaces and no `sc-lint-version`-owned
    HTML renderer
  - see [docs/phase-C/sprint-C2.md](./sprint-C2.md)
- `C.3`
  - hard-fail version gate planning
  - multi-interface-family verdict model, semver-ingestion contract, and
    CI/developer workflow wiring
  - see [docs/phase-C/sprint-C3.md](./sprint-C3.md)
- `C.4`
  - consumer integration documentation and Claude Code skill planning
  - explicit “what a consuming repo must do” scope
  - authoritative adoption doc:
    `docs/sc-lint/version-adoption.md`
  - repo-local adoption skill:
    `.claude/skills/sc-lint-version-adoption/SKILL.md`
  - see [docs/phase-C/sprint-C4.md](./sprint-C4.md)
- `C.5`
  - minimal repo-local Claude Code marketplace planning for the adoption skill
  - explicit forwarding/reference path for marketplace advertisement
  - see [docs/phase-C/sprint-C5.md](./sprint-C5.md)
- `C.6`
  - production path-literal portability parity in `sc-lint-portability`
  - Unix-only and Windows-only absolute path literals in production code
  - see [docs/phase-C/sprint-C6.md](./sprint-C6.md)
- `C.7`
  - broad production environment-variable portability in
    `sc-lint-portability`
  - `HOME`, `USER`, and `XDG_*` portability checks with platform-neutral
    remediation guidance
  - see [docs/phase-C/sprint-C7.md](./sprint-C7.md)
- `C.8`
  - shared shell invocation portability in `sc-lint-portability`
  - `Command::new("sh" | "bash")` and hardcoded `/bin/sh` or `/bin/bash`
    production checks
  - implemented on `feature/sprint-C8`
  - see [docs/phase-C/sprint-C8.md](./sprint-C8.md)
- `C.9`
  - structural cross-platform `cfg` parity enforcement in
    `sc-lint-portability`
  - production `#[cfg(unix)]` branches that lack Windows companions or
    explicit portable fallbacks
  - see [docs/phase-C/sprint-C9.md](./sprint-C9.md)
- `C.10`
  - `sc-observability` `1.1.0` adoption in the CLI-owned logging layer
  - retained-log policy decision, `emit` -> `log` / `try_log` migration, and
    Windows rotation validation target
  - see [docs/phase-C/sprint-C10.md](./sprint-C10.md)

## Phase Structure

1. `C.1`
   - decide what is being versioned
   - decide what counts as a breaking change
   - lock the initial Rust public API engine, command surface, and
     family-selection configuration strategy
2. `C.2`
   - define the shared report system consumed by interface-version artifacts
   - lock the structured artifact schema, XHTML fragment/report pattern,
     `sc-compose`/Jinja template direction, and template-override workflow
3. `C.3`
   - define how version checks hard-fail and where those checks run
   - connect the per-family artifact model to repo gates and release review
4. `C.4`
   - define how consuming repos adopt the capability
   - package the adoption guidance as a repo-local skill
   - keep `docs/sc-lint/version-adoption.md` authoritative for the adoption
     workflow
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
10. `C.10`
   - adopt `sc-observability` `1.1.0`
   - decide retained-log policy rollout for the current release line
   - migrate deprecated `emit` call sites to `log` / `try_log`
   - verify typestate-compatible shutdown and Windows rotation assumptions

## Exit Direction

Phase `C` should leave the repo with:

- an approved versioning approach for Rust public APIs based on
  `cargo-semver-checks`
- an accepted form-factor decision for a dedicated `sc-lint-version` crate
  invoked through `sc-lint check interfaces`
- explicit requirements for CLI and RPC/socket breaking-change detection
- a shared reporting decision that keeps reusable HTML/XHTML generation out of
  `sc-lint-version` itself and instead targets the `sc-compose` orbit as the
  preferred home for the reusable reporting layer, potentially as a dedicated
  `sc-reporting` capability
- a generated artifact model for published interface documentation:
  - main HTML report
  - JSON sidecar
  - separate XHTML section fragments/panels with built-in copy actions
- a template family plan for:
  - Rust public API reports
  - CLI contract reports
  - ICD-style RPC/socket reports
- a plan for published interface coverage across all shipped crates
- a defined configuration surface under `[version.families.<family>]` for
  selecting interface families and baselines
- a defined configuration surface under `[reporting.templates.<report_kind>]`
  for selecting or overriding report templates without patching generated HTML
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
- one explicit plan for `sc-observability` `1.1.0` adoption in the top-level
  CLI logging layer, including:
  - typestate-compatible shutdown verification
  - retained-log policy enable/defer decision
  - deprecated `emit` call-site migration to `log` / `try_log`
  - explicit `sc-observe` adoption decision
