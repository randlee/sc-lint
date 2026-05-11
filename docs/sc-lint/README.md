# `sc-lint` Docs

This folder is the home for `sc-lint` design and planning material.

Current contents:

- [`requirements.md`](./requirements.md) — consumer-neutral requirements for
  boundary-source migration and inventory-parity behavior
- [`mvp.md`](./mvp.md) — MVP design for the initial `sc-lint-boundary`
  analyzer and the paired `sc-lint-attributes` plan
- [`roadmap.md`](./roadmap.md) — decisions, rollout sequence, and what stays in
  Python vs what moves to Rust
- [`graph-schema.md`](./graph-schema.md) — current graph/export contract and
  rule-id inventory
- [`boundary-enforcement-model.md`](./boundary-enforcement-model.md) — planned
  warn/error escalation model for inventory-parity boundary enforcement
- [`boundary-toml-migration.md`](./boundary-toml-migration.md) — migration plan
  for moving canonical boundary data from Markdown-embedded records to
  standalone TOML
- [`extraction-plan.md`](./extraction-plan.md) — extraction plan for remaining
  generic lint/view tooling and the Python-to-Rust boundary migration
- [`foundation-phase-plan.md`](./foundation-phase-plan.md) — current detailed
  execution plan for repo self-hosting, boundaries, CLI introduction, and
  extraction order
- [`phase-B-plan.md`](./phase-B-plan.md) — current Phase B execution stub and
  post-mortem carry-forward planning line
- [`crate-architecture.md`](./crate-architecture.md) — crate-by-crate role,
  ownership, and Phase A touchpoint guide
- [`adr/README.md`](./adr/README.md) — ADR index for the current architecture
  decisions
- [`sprint-A1a.md`](./sprint-A1a.md) — top-level CLI bootstrap and contract
  definition sprint
- [`sprint-A1b.md`](./sprint-A1b.md) — top-level config loading and first
  backend integration sprint
- [`sprint-A2.md`](./sprint-A2.md) — profiles and `xwin` sprint
- [`sprint-A3.md`](./sprint-A3.md) — generic utility extraction sprint
- [`sprint-A4.md`](./sprint-A4.md) — portability crate extraction sprint
- [`sprint-A5.md`](./sprint-A5.md) — runtime crate extraction sprint
- [`sprint-A6.md`](./sprint-A6.md) — Rust boundary inventory loader sprint
- [`sprint-A7.md`](./sprint-A7.md) — manifest-policy and parity sprint
- [`sprint-A8.md`](./sprint-A8.md) — per-tool user-guide sprint
- [`sprint-B1.md`](./sprint-B1.md) — post-mortem carry-forward and systemic
  lint-gate planning sprint
- [`cli-requirements.md`](./cli-requirements.md) — detailed requirements for
  the planned top-level `sc-lint` CLI
- [`cli-architecture.md`](./cli-architecture.md) — detailed architecture for
  the planned top-level `sc-lint` CLI
- [`cli-contract.md`](./cli-contract.md) — planned top-level success/error
  envelope and backend-to-CLI normalization contract
- [`logging.md`](./logging.md) — structured logging design, rollout, and event
  schema for the top-level CLI

Current intended crate split:

- `sc-lint`
  - planned top-level CLI crate
  - command parsing, config loading, output normalization, tool dispatch
  - canonical AI-first machine contract for non-interactive commands
  - planned profiles:
    - `fast`
    - `full`
    - `ci`
  - planned top-level CI-equivalent command:
    - `sc-lint ci`
  - planned Windows preflight commands when `cargo xwin` is installed:
    - `sc-lint check xwin`
    - `sc-lint clippy xwin`
- `sc-lint-directives`
  - shared directive parsing/types
- `sc-lint-boundary`
  - analyzer CLI + library
  - AST parsing, graph construction, semantic boundary rule evaluation
- `sc-lint-portability`
  - planned analyzer crate for shared OS/platform portability rules
- `sc-lint-runtime`
  - planned analyzer crate for shared std runtime/concurrency rules
- `sc-lint-tokio`
  - planned future analyzer crate for Tokio-specific rules
  - represented now as a reserved future boundary surface only
- `sc-lint-attributes`
  - proc-macro attribute crate
  - intentionally minimal at first
  - exists early so source-level declarations can be added without late
    packaging churn

Current scaffold status:

- `sc-lint-directives`
  - exists now
  - currently shares the workspace `0.1.0` version line
  - currently provides shared parsing for `#[sc_lint(...)]` directives
- `sc-lint-attributes`
  - exists now
  - currently shares the workspace `0.1.0` version line
  - currently provides compile-valid, no-op `#[sc_lint(...)]` support for:
    - `boundary.allow("cycle.type_method_self_loop")`
    - `boundary.allow("cycle.recursive_value_container")`
    - `boundary.internal_only`
- `sc-lint-boundary`
  - exists now
  - currently shares the workspace `0.1.0` version line
  - currently provides:
    - workspace discovery through `cargo_metadata`
    - module-driven source traversal through `syn`
    - graph nodes for:
      - crates
      - modules
      - types
      - impls
      - variants
      - fields
      - traits
      - trait references
      - functions
      - methods
    - `#[sc_lint(...)]` attribute ingestion for `boundary.allow(...)` and
      `boundary.internal_only`
    - owner-graph cycle classification with:
      - `SCB-CYCLE-001` multi-owner architectural cycle
      - `SCB-CYCLE-002` type/method self-loop
      - `SCB-CYCLE-003` trait-impl self-loop
    - built-in default trait-self-loop policy from:
      - `crates/sc-lint-boundary/config/defaults.toml`
    - boundary enforcement with:
      - `SCB-BOUNDARY-001` internal_only visibility violation
      - `SCB-BOUNDARY-002` internal_only external reference
      - `SCB-BOUNDARY-003` forbid_external_impls violation
    - temporary portability enforcement inside `sc-lint-boundary` with:
      - `PORT-001` hardcoded Unix-only absolute paths in test code
      - `PORT-002` direct `dirs::home_dir()` without configured override check
      - `PORT-003` `std::env::set_var()` in test code
    - stable text/JSON findings output
    - graph export in:
      - JSON
      - Turtle
- `sc-lint`
  - planned now
  - not implemented yet
  - detailed CLI requirements and architecture are defined in:
    - [`cli-requirements.md`](./cli-requirements.md)
    - [`cli-architecture.md`](./cli-architecture.md)

Current code moves required for the planned partition:

- move portability rules out of `crates/sc-lint-boundary/src/portability.rs`
  into the future `sc-lint-portability` crate:
  - `PORT-001`
  - `PORT-002`
  - `PORT-003`
  - `PORT-004`
  - `PORT-005`
- import std runtime/concurrency rules from the current `atm-core` proving
  surface into the future `sc-lint-runtime` crate:
  - `SCB-RUNTIME-001`
  - `SCB-RUNTIME-002`
- retarget the current portability wrapper surface when that crate exists:
  - `.just/lint_sc_portability.py`
  - `.just/run_lint.py`

Planned primary lint-target mapping for the top-level CLI:

- `sc-lint lint sc-boundary`
  - backend owner: `sc-lint-boundary`
- `sc-lint lint sc-portability`
  - backend owner: `sc-lint-portability`
- `sc-lint lint sc-runtime`
  - backend owner: `sc-lint-runtime`

Grouped subset aliases may exist later, but these crate-mapped targets are the
primary ownership-preserving command surface.

Planned next shared rule imports from `atm-core`:

- `sc-lint-portability`
  - `PORT-004`
  - `PORT-005`
- `sc-lint-runtime`
  - `SCB-RUNTIME-001`
  - `SCB-RUNTIME-002`

Kept local to consumer repos for now:

- duplicate semantic string-literal policy
- fixed-sleep test-hygiene policy
- TTL triage consistency policy

Current repo integration status:

- `just lint sc-boundary`
  - exists now as a named target
  - is part of default `just lint` for this repo
- `just lint sc-portability`
  - exists now as a named target
  - is part of default `just lint` for this repo

Current planned profile policy:

- `fast`
  - local low-latency lint profile
  - excludes `xwin` to preserve low-latency local feedback
- `full`
  - stronger local pre-push lint profile
  - includes `xwin check` and `xwin clippy` when available
- `ci`
  - lint-only CI-parity profile
  - excludes `xwin`
- top-level `ci`
  - lint plus tests

Current repo boundary source status:

- canonical boundary TOML is expected under `boundaries/`
- `sc-lint` crate boundaries are now defined there for current phase planning
- default lint enforcement against those records is scheduled to land with the
  Rust boundary inventory loader migration

Planned rule families not implemented yet:

- `SCB-INVENTORY-001`
- `SCB-INVENTORY-002`
- `SCB-INVENTORY-003`

Planned canonical boundary layout:

```text
boundaries/
  consumer-core/
    mail-store.toml
    identity-registry.toml
  runtime-service/
    server-transport.toml
  planning.toml
```

That layout remains the generic long-term pattern. This repo now also uses the
same canonical `boundaries/` root for its own current and planned tool
surfaces.

Future documents that should also live here:

- crate layout
- rule inventory
- deeper RDF/Oxygraph integration notes
- release-1 acceptance notes

Related architecture decisions:

- [`./adr/ADR-004-structured-boundary-definitions.md`](./adr/ADR-004-structured-boundary-definitions.md)
  — canonical TOML boundary source plus planning-aware inventory-parity
- [`./adr/ADR-005-cli-profiles-and-xwin-preflight.md`](./adr/ADR-005-cli-profiles-and-xwin-preflight.md)
  — top-level CLI profile semantics plus capability-driven `xwin` preflight
- [`./adr/ADR-006-ai-first-cli-contract.md`](./adr/ADR-006-ai-first-cli-contract.md)
  — top-level CLI as the stable machine-contract owner rather than a
  dispatcher-only wrapper
- [`./adr/ADR-007-analyzer-crate-partition.md`](./adr/ADR-007-analyzer-crate-partition.md)
  — analyzer-crate partitioning and primary lint-target mapping
- [`./adr/ADR-008-sc-observability-logging.md`](./adr/ADR-008-sc-observability-logging.md)
  — `sc-observability` selection plus CLI-owned structured logging policy

Planned A.8 user-guide convention:

- per-tool guides will live under `docs/sc-lint/tools/`
- each file will be named after the tool it documents
- the repository-root `README.md` will link every guide directly
