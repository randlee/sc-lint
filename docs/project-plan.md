# sc-lint Project Plan

This document is the high-level planning index for the `sc-lint` project.

## Current Planning Scope

The current project focus is:

- extracting `sc-lint` into a standalone repository
- stabilizing the initial crate set
- defining canonical crate boundaries for `sc-lint` itself
- establishing a self-hosting `just lint` development gate for this repo
- planning and introducing a top-level `sc-lint` CLI
- defining the top-level CLI as an AI-first machine contract rather than only
  a human wrapper over backend tools
- migrating boundary-definition handling to structured TOML sources
- migrating generic lint/view tooling into `sc-lint`
- moving boundary inventory and manifest-policy enforcement from Python into
  `sc-lint-boundary`
- backporting reusable lint families that were first proven on `atm-core`
- splitting imported lint families into narrowly-scoped analyzer crates rather
  than growing catch-all backends
- keeping consumer-specific policy lints local unless only their framework is
  worth extracting
- planning stable interface version checks and shared report publication for
  public APIs, CLI contracts, and transport surfaces
- planning the next shared production portability follow-ons in
  `sc-lint-portability`: production path literals, broad env portability,
  shell invocation portability, and structural `cfg(unix)` parity
- improving pre-CI developer confidence with cross-target compile checks where
  those checks can surface platform drift before a push
- preserving CI and lint-runner parity during extraction

## Current Detailed Planning References

- project roadmap
  - see [docs/sc-lint/roadmap.md](./sc-lint/roadmap.md)
- boundary TOML migration plan
  - see [docs/sc-lint-boundary/boundary-toml-migration.md](./sc-lint-boundary/boundary-toml-migration.md)
- boundary enforcement model rollout
  - see [docs/sc-lint-boundary/boundary-enforcement-model.md](./sc-lint-boundary/boundary-enforcement-model.md)
- CLI requirements and contract
  - see [docs/sc-lint/cli-requirements.md](./sc-lint/cli-requirements.md)
  - see [docs/sc-lint/cli-architecture.md](./sc-lint/cli-architecture.md)
  - see [docs/sc-lint/cli-contract.md](./sc-lint/cli-contract.md)
  - see [docs/sc-lint/crate-architecture.md](./sc-lint/crate-architecture.md)
  - see [docs/sc-lint/adr/README.md](./sc-lint/adr/README.md)
  - see [docs/sc-lint/logging.md](./sc-lint/logging.md)
  - see [docs/sc-lint/adr/ADR-008-sc-observability-logging.md](./sc-lint/adr/ADR-008-sc-observability-logging.md)
  - see [docs/sc-lint/adr/ADR-009-observability-boundary-policy.md](./sc-lint/adr/ADR-009-observability-boundary-policy.md)
  - see [docs/sc-lint/adr/ADR-007-analyzer-crate-partition.md](./sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- extraction and migration plan
  - see [docs/sc-lint/extraction-plan.md](./sc-lint/extraction-plan.md)
- known issues inventory
  - see [docs/issues-inventory.md](./issues-inventory.md)
- current phase execution plan
  - see [docs/phase-A/foundation-phase-plan.md](./phase-A/foundation-phase-plan.md)
  - see [docs/phase-B/phase-B-plan.md](./phase-B/phase-B-plan.md)
  - see [docs/phase-C/phase-C-plan.md](./phase-C/phase-C-plan.md)
  - see [docs/phase-A/sprint-A1a.md](./phase-A/sprint-A1a.md)
  - see [docs/phase-A/sprint-A1b.md](./phase-A/sprint-A1b.md)
  - see [docs/phase-A/sprint-A2.md](./phase-A/sprint-A2.md)
  - see [docs/phase-A/sprint-A3.md](./phase-A/sprint-A3.md)
  - see [docs/phase-A/sprint-A4.md](./phase-A/sprint-A4.md)
  - see [docs/phase-A/sprint-A5.md](./phase-A/sprint-A5.md)
  - see [docs/phase-A/sprint-A6.md](./phase-A/sprint-A6.md)
  - see [docs/phase-A/sprint-A7.md](./phase-A/sprint-A7.md)
  - see [docs/phase-A/sprint-A8.md](./phase-A/sprint-A8.md)
- initial analyzer MVP
  - see [docs/sc-lint/mvp.md](./sc-lint/mvp.md)

## Current Phase Priorities

This phase should execute in the following order:

1. define canonical `sc-lint` boundaries in TOML
2. make `just lint` self-host the repo's own analyzer checks by default
3. add the top-level `sc-lint` CLI crate and define its canonical machine
   contract
4. complete the A.1a exit review of the CLI contract against the needs of
   extracted Python utilities and later analyzer backends before A.1b starts
5. add top-level config loading and the first delegated backend path
6. define the cross-target preflight strategy for local and CI lint flows
7. extract generic Python utilities
8. add the next analyzer crate needed for portability rule-family ownership
9. move portability rules into `sc-lint-portability`
10. add the next analyzer crate needed for std runtime rule-family ownership
11. import runtime rules into `sc-lint-runtime`
12. migrate boundary inventory loading/schema/duplicate handling from Python
    into `sc-lint-boundary`
13. migrate manifest policy into `sc-lint-boundary`
14. run parity validation before deprecating Python boundary logic
15. publish comprehensive per-tool user guides and rule-disable guidance for
    the release-1 lint surface

## Scheduled Sprint Plans

The currently scheduled sprint plans are:

- `A.1a`
  - CLI bootstrap and contract definition
  - includes the A.1a exit-review checkpoint for Workstreams 4-7
  - `docs/phase-A/sprint-A1a.md`
- `A.1b`
  - config loading and first backend integration
  - first operational path is `sc-lint lint sc-boundary`
  - `docs/phase-A/sprint-A1b.md`
- `A.2`
  - profiles and Windows preflight
  - active implementation branch: `feature/sprint-A2`
  - `docs/phase-A/sprint-A2.md`
- `A.3`
  - generic utility extraction
  - active implementation branch: `feature/sprint-A3`
  - `docs/phase-A/sprint-A3.md`
- `A.4`
  - `sc-lint-portability` crate creation and portability-rule moves/imports
  - current status: complete
  - `docs/phase-A/sprint-A4.md`
- `A.5`
  - `sc-lint-runtime` crate creation and runtime-rule imports
  - active implementation branch: `feature/sprint-A5`
  - `docs/phase-A/sprint-A5.md`
- `A.6`
  - Rust boundary inventory loading, schema validation, and duplicate handling
  - active implementation branch: `feature/sprint-A6`
  - `docs/phase-A/sprint-A6.md`
- `A.7`
  - Rust manifest-policy enforcement and Python parity window
  - active implementation branch: `feature/sprint-A7`
  - `docs/phase-A/sprint-A7.md`
- `A.8`
  - per-tool user guides and rule-disable documentation
  - active implementation branch: `feature/sprint-A8`
  - `docs/phase-A/sprint-A8.md`
- `B.1`
  - carry-forward lint-gate and portability-scope hardening
  - active implementation branch: `feature/sprint-B1`
  - explicit backlog planning for seven recurring shared lint-gate families:
    identity literals, `/tmp/` paths, public API `anyhow::Error`, duplicated
    `CrateId` newtypes, `for_kv_map`-style loops, over-broad `pub`, and raw
    `String` structured identifiers
  - explicit backlog planning for shared portability follow-ons in
    `sc-lint-portability`: Windows-path parity, broader env portability, and
    shell portability
  - `docs/phase-B/sprint-B1.md`
- `B.2`
  - named-caller allowlist enforcement in `sc-lint-boundary`
  - `docs/phase-B/sprint-B2.md`
- `B.3`
  - observability boundary-policy ADR acceptance, including promotion of
    `ADR-009` from stub to accepted policy text plus boundary/planning
    alignment for the approved CLI-owned observability seams
  - `docs/phase-B/sprint-B3.md`
- `B.4`
  - QA-process hardening
  - triage-first fix routing plus regression-tested TODO/carry-forward tooling
  - `docs/phase-B/sprint-B4.md`
- `sprint-B-homebrew`
  - full `sc-lint` Homebrew toolset distribution planning
  - sprint number intentionally pending
  - `docs/phase-B/sprint-B-homebrew.md`
- `C.1`
  - `sc-lint-version` policy and baseline definition
  - dedicated crate/form-factor, invocation command, and family-selection
    configuration surface
  - `docs/phase-C/sprint-C1.md`
- `C.2`
  - shared report-template pipeline
  - versioned CLI baseline artifact schema, `sc-compose`/Jinja template
    direction, and generation workflow
  - `docs/phase-C/sprint-C2.md`
- `C.3`
  - hard-fail version gate integration
  - cargo-semver-checks ingestion and multi-family verdict wiring
  - `docs/phase-C/sprint-C3.md`
- `C.4`
  - consumer integration and skill design
  - `docs/phase-C/sprint-C4.md`
- `C.5`
  - minimal marketplace publication for the adoption skill
  - `docs/phase-C/sprint-C5.md`
- `C.6`
  - production path-literal portability parity
  - `PORT-006` Unix-only absolute path literals in production code
  - `PORT-007` Windows-only absolute path literals in production code
  - `docs/phase-C/sprint-C6.md`
- `C.7`
  - broad environment-variable portability
  - `PORT-008` production `HOME`, `USER`, and `XDG_*` portability checks
  - `docs/phase-C/sprint-C7.md`
- `C.8`
  - shell invocation portability
  - `PORT-009` production `sh`/`bash` and `/bin/sh` shell assumption checks
  - `docs/phase-C/sprint-C8.md`
- `C.9`
  - cross-platform `cfg` parity enforcement
  - `PORT-010` production `#[cfg(unix)]` companion-parity checks
  - `docs/phase-C/sprint-C9.md`
- `C.10`
  - `sc-observability` `1.1.0` adoption
  - retained-log policy enablement plus typestate, supported `emit(...)`
    compatibility, and Windows-rotation compatibility review for the CLI-owned
    logging seam
  - `docs/phase-C/sprint-C10.md`

## Recent Sprint Deltas

- `A.6`
  - Rust-native TOML boundary inventory loading, schema validation, and
    duplicate handling now live in `sc-lint-boundary`
  - the Python parity oracle now exists at `.just/lint_boundaries.py`
  - fixture coverage now includes valid, invalid-schema, and duplicate
    inventory cases in `.just/tests/test_lint_boundaries.py`
- `A.8`
  - per-tool user guides now live under `docs/sc-lint/tools/`
  - direct guide links are now published from both `README.md` and
    `docs/sc-lint/README.md`
- `B.4`
  - triage-first QA routing is now the authoritative default before fix
    dispatch
  - QA-2+ now stays in targeted-fix mode with QA-1-only default
    `rust-best-practices`
  - TODO discovery and carry-forward routing helpers now have explicit
    regression coverage in `scripts/test_find_todos.py` and
    `scripts/test_triage_carry_forward.py`

## Next Analyzer-Crate Additions

The next planned tool crates after the current line are:

- `sc-lint-portability`
  - first moves/imports:
    - `PORT-001`
    - `PORT-002`
    - `PORT-003`
    - `PORT-004`
    - `PORT-005`
- `sc-lint-runtime`
  - first moves/imports:
    - `SCB-RUNTIME-001`
    - `SCB-RUNTIME-002`
- `sc-lint-tokio`
  - planned crate reservation only for now
  - no initial implementation scope until Tokio-specific rules justify it
- `sc-lint-version`
  - planned dedicated interface-versioning crate
  - governing sprint sequence:
    - `C.1`
    - `C.2`
    - `C.3`
    - `C.4`
    - `C.5`

## Release 1 Target

Release `0.2.x` should establish:

- a stable repo-local development and CI gate
- canonical TOML boundaries for the repo's own tool surfaces
- canonical TOML boundaries for the planned top-level CLI contract items
- a documented and approved top-level `sc-lint` CLI contract
- explicit machine-contract decisions for:
  - canonical `--json` mode
  - one envelope and error pattern for every non-interactive command family
  - stable machine-readable failures
  - reusable request/response seams
  - secondary interactive graph surfaces only
- a detailed extraction and migration plan for remaining generic tooling
- an explicit release-1 scope statement for which product surfaces are and are
  not boundary-inventory enforced
- a documented partition for:
  - reusable analyzer families that migrate into `sc-lint`
  - consumer-local policy lints that stay in their proving repo
- a documented strategy for surfacing likely Windows/Linux compile failures
  before CI without pretending that cross-target checks replace real
  multi-platform runners
- an initial Windows preflight path based on `cargo xwin check`, with a clear
  stance on whether and when `cargo xwin clippy` should be promoted
- documented `fast/full/ci` profile semantics and the distinction between:
  - `sc-lint lint ci`
  - `sc-lint ci`
- an implementation path for moving boundary inventory and manifest-policy
  logic into Rust with Python parity validation retained during migration
- comprehensive user guides for each shipped linter tool, including:
  - how the tool is invoked
  - representative examples
  - how rules are disabled or scoped out when policy permits
  - one document per tool named after the lint tool and linked from the
    repository-root `README.md`

Phase `A` completed the release-1 foundation implementation line. Phase `B` is
the current planning and hardening line for post-Phase-A defect prevention,
follow-on feature work, and release/distribution planning. That does not imply
that every release-1 follow-on item is already complete.

Phase `C` is now the next queued planning line after the current Phase `B`
sequence. It covers `sc-lint-version`, the shared reporting pipeline it
consumes, and multi-surface breaking-change detection.

## Planning Conventions

- This file tracks project-level phases and priorities.
- Detailed sprint, phase, or feature plans should live under `docs/sc-lint/`
  unless they are repo-wide concerns.
- New crate introductions and major lint families should be added here as
  top-level planning items and then linked to their detailed plan documents.
