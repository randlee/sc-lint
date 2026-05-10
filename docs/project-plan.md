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
- improving pre-CI developer confidence with cross-target compile checks where
  those checks can surface platform drift before a push
- preserving CI and lint-runner parity during extraction

## Current Detailed Planning References

- project roadmap
  - see [docs/sc-lint/roadmap.md](./sc-lint/roadmap.md)
- boundary TOML migration plan
  - see [docs/sc-lint/boundary-toml-migration.md](./sc-lint/boundary-toml-migration.md)
- boundary enforcement model rollout
  - see [docs/sc-lint/boundary-enforcement-model.md](./sc-lint/boundary-enforcement-model.md)
- CLI requirements and contract
  - see [docs/sc-lint/cli-requirements.md](./sc-lint/cli-requirements.md)
  - see [docs/sc-lint/cli-architecture.md](./sc-lint/cli-architecture.md)
  - see [docs/sc-lint/cli-contract.md](./sc-lint/cli-contract.md)
  - see [docs/sc-lint/crate-architecture.md](./sc-lint/crate-architecture.md)
  - see [docs/sc-lint/adr/README.md](./sc-lint/adr/README.md)
  - see [docs/sc-lint/logging.md](./sc-lint/logging.md)
  - see [docs/sc-lint/adr/ADR-008-sc-observability-logging.md](./sc-lint/adr/ADR-008-sc-observability-logging.md)
  - see [docs/sc-lint/adr/ADR-007-analyzer-crate-partition.md](./sc-lint/adr/ADR-007-analyzer-crate-partition.md)
- extraction and migration plan
  - see [docs/sc-lint/extraction-plan.md](./sc-lint/extraction-plan.md)
- known issues inventory
  - see [docs/issues-inventory.md](./issues-inventory.md)
- current phase execution plan
  - see [docs/sc-lint/foundation-phase-plan.md](./sc-lint/foundation-phase-plan.md)
  - see [docs/sc-lint/sprint-A1a.md](./sc-lint/sprint-A1a.md)
  - see [docs/sc-lint/sprint-A1b.md](./sc-lint/sprint-A1b.md)
  - see [docs/sc-lint/sprint-A2.md](./sc-lint/sprint-A2.md)
  - see [docs/sc-lint/sprint-A3.md](./sc-lint/sprint-A3.md)
  - see [docs/sc-lint/sprint-A4.md](./sc-lint/sprint-A4.md)
  - see [docs/sc-lint/sprint-A5.md](./sc-lint/sprint-A5.md)
  - see [docs/sc-lint/sprint-A6.md](./sc-lint/sprint-A6.md)
  - see [docs/sc-lint/sprint-A7.md](./sc-lint/sprint-A7.md)
  - see [docs/sc-lint/sprint-A8.md](./sc-lint/sprint-A8.md)
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

The currently scheduled foundation sprints are:

- `A.1a`
  - CLI bootstrap and contract definition
  - includes the A.1a exit-review checkpoint for Workstreams 4-7
  - `docs/sc-lint/sprint-A1a.md`
- `A.1b`
  - config loading and first backend integration
  - first operational path is `sc-lint lint sc-boundary`
  - `docs/sc-lint/sprint-A1b.md`
- `A.2`
  - profiles and Windows preflight
  - active implementation branch: `feature/sprint-A2`
  - `docs/sc-lint/sprint-A2.md`
- `A.3`
  - generic utility extraction
  - active implementation branch: `feature/sprint-A3`
  - `docs/sc-lint/sprint-A3.md`
- `A.4`
  - `sc-lint-portability` crate creation and portability-rule moves/imports
  - `docs/sc-lint/sprint-A4.md`
- `A.5`
  - `sc-lint-runtime` crate creation and runtime-rule imports
  - `docs/sc-lint/sprint-A5.md`
- `A.6`
  - Rust boundary inventory loading, schema validation, and duplicate handling
  - active implementation branch: `feature/sprint-A6`
  - `docs/sc-lint/sprint-A6.md`
- `A.7`
  - Rust manifest-policy enforcement and Python parity window
  - `docs/sc-lint/sprint-A7.md`
- `A.8`
  - per-tool user guides and rule-disable documentation
  - `docs/sc-lint/sprint-A8.md`

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

## Release 1 Target

Release `0.1.x` should establish:

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

The current phase, Phase `A`, is the release-1 foundation phase. It does not imply that
every release-1 implementation item is already complete.

## Planning Conventions

- This file tracks project-level phases and priorities.
- Detailed sprint, phase, or feature plans should live under `docs/sc-lint/`
  unless they are repo-wide concerns.
- New crate introductions and major lint families should be added here as
  top-level planning items and then linked to their detailed plan documents.
