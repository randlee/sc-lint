# sc-lint Foundation Phase Plan

This document is the detailed execution plan for the current `sc-lint` phase,
Phase `A`.

## Objective

Turn `sc-lint` from an extracted code workspace into a self-hosting lint tool
repo with:

- canonical crate boundaries
- a default local development gate that exercises the repo's own analyzer
- a concrete plan for the top-level `sc-lint` CLI
- a sequenced migration path for generic Python utilities and boundary logic
- an explicit path for promoting reusable lint families from `atm-core`

## Workstreams

### Workstream 1: Repo boundaries

Define canonical TOML boundary records for the current `sc-lint` crates and the
planned top-level CLI.

Required outputs:

- `boundaries/sc-lint-directives/*.toml`
- `boundaries/sc-lint-attributes/*.toml`
- `boundaries/sc-lint-boundary/*.toml`
- `boundaries/sc-lint-portability/*.toml`
- `boundaries/sc-lint-runtime/*.toml`
- `boundaries/sc-lint-tokio/*.toml`
- `boundaries/sc-lint/*.toml`
- `boundaries/planning.toml`

Required policy intent:

- `sc-lint-directives` is the shared directive parser crate
- `sc-lint-attributes` depends on `sc-lint-directives`
- `sc-lint-boundary` depends on `sc-lint-directives`
- `sc-lint-portability` is a separate analyzer crate for portability rules
- `sc-lint-runtime` is a separate analyzer crate for std runtime rules
- `sc-lint-tokio` is a reserved future analyzer crate for Tokio-specific rules
- future `sc-lint` CLI coordinates backend tools but does not force backend
  crate cross-dependencies

Current phase note:

- these boundary files are canonical planning inputs now
- default lint enforcement against them activates when boundary inventory
  loading is moved into `sc-lint-boundary`

### Workstream 2: Self-hosting lint gate

Make `just lint` reflect the development needs of this repo.

Required changes:

- keep generic repo gates:
  - fmt
  - clippy
  - deny
  - shear
  - version
  - manifests
  - spell
  - pytests
- include:
  - `sc-boundary`
  - `sc-portability`
  in the default local lint gate for this repo
- keep `modules` advisory/manual

Acceptance criteria:

- `just lint` passes on `sc-lint`
- `just lint sc-boundary` passes
- `just lint sc-portability` passes

### Workstream 3: Top-level CLI

Introduce the top-level `sc-lint` CLI as the stable user-facing entry point.

Phase goals:

- create the crate
- define command structure
- define the canonical machine-readable contract
- define output and exit-code conventions
- load config once at the top level
- dispatch to self-contained backends

Initial command shape:

- `sc-lint lint <tool>`
- `sc-lint view <tool>`
- `sc-lint version`
- `sc-lint ci`

Primary planned crate-mapped lint targets:

- `sc-lint lint sc-boundary`
- `sc-lint lint sc-portability`
- `sc-lint lint sc-runtime`

Subset aliases may exist later, but the primary command surface should track
backend-crate ownership directly.

For release `0.1.x`, the `view` family remains narrower than `lint`:

- view targets are documented individually before exposure
- they may remain backed by repo-local Python/report plumbing
- they are not required to map one-to-one to backend analyzer crates

Initial profile direction:

- `sc-lint lint fast`
- `sc-lint lint full`
- `sc-lint lint ci`

Initial Windows-preflight direction when `xwin` is installed:

- `sc-lint check xwin`
- `sc-lint clippy xwin`

Required contract decisions:

- top-level machine mode is `--json`
- non-interactive commands remain machine-readable on both success and failure
- request/response seams stay reusable outside the CLI entrypoint
- human-readable output remains a presentation layer, not the only tested
  interface
- future interactive graph features remain secondary surfaces

### Workstream 4: Generic Python utility extraction

Extract the current consumer-neutral Python tools into `sc-lint`.

Priority order:

1. line-count lint
2. identity literal lint
3. generic view plumbing

Requirements:

- consumer-neutral naming
- standalone fixture tests
- top-level CLI exposure once the CLI exists

### Workstream 5: Boundary logic migration to Rust

Move boundary inventory and manifest-policy enforcement from Python into
`sc-lint-boundary`.

Required phases:

1. TOML boundary inventory loading
2. schema validation
3. duplicate-source/id handling
4. manifest ownership rules
5. manifest section rules
6. parity validation against the Python implementation

### Workstream 6: Add `sc-lint-portability`

Create the dedicated portability analyzer crate and move all shared
portability rules into it.

Current in-scope rules:

- `PORT-001`
- `PORT-002`
- `PORT-003`
- `PORT-004`
- `PORT-005`

Requirements:

- create `sc-lint-portability`
- move the existing portability implementation out of `sc-lint-boundary`
- land portability rules in `sc-lint-portability`
- preserve their existing rule ids

### Workstream 7: Add `sc-lint-runtime`

Create the dedicated std runtime/concurrency analyzer crate and import the
shared runtime rules into it from the consumer-repo proving surface.

Current in-scope rules:

- `SCB-RUNTIME-001`
- `SCB-RUNTIME-002`

Requirements:

- create `sc-lint-runtime`
- land runtime rules in `sc-lint-runtime`
- preserve their existing rule ids
- keep ATM-local policy lints out of `sc-lint` unless extracted only as
  configurable framework

### Workstream 8: Cross-target preflight strategy

Define how `sc-lint` should help developers surface likely Windows/Linux
compile drift before CI while still preserving the distinction between
preflight and true multi-platform validation.

Required work:

- document the supported cross-target preflight mode
- document `cargo xwin` as the first Windows preflight candidate
- determine the profile policy:
  - `fast` may include `xwin check`
  - `full` may include `xwin check` and `xwin clippy`
  - `ci` excludes `xwin`
- determine whether `cargo xwin clippy` remains an explicit stronger path
  rather than part of the default gate
- document the difference between:
  - compile-time drift detection
  - authoritative native-platform CI validation
  - `sc-lint lint ci` and top-level `sc-lint ci`

Current constraint:

- do not present cross-target preflight as a replacement for Windows/macOS/Linux
  CI runners

## Sequence

The current phase should execute in this order:

1. define repo boundaries
2. tighten `just lint` for self-hosting
3. add the top-level `sc-lint` CLI
4. define the cross-target preflight strategy
5. extract generic Python utilities
6. add `sc-lint-portability` and move shared portability rules into it
7. add `sc-lint-runtime` and import shared std runtime rules into it
8. migrate boundary inventory loading/schema/duplicate handling into Rust
9. migrate manifest-policy logic into Rust
10. keep Python parity validation during the migration window

## Planned Sprint Sequence

The scheduled implementation sprints for this phase are:

1. `A.1`
   - top-level CLI bootstrap
   - sprint plan: `docs/sc-lint/sprint-A1.md`
2. `A.2`
   - profile semantics and `xwin` capability support
   - sprint plan: `docs/sc-lint/sprint-A2.md`
3. `A.3`
   - generic utility extraction
   - sprint plan: `docs/sc-lint/sprint-A3.md`
4. `A.4`
   - `sc-lint-portability` creation and portability-rule migration
   - sprint plan: `docs/sc-lint/sprint-A4.md`
5. `A.5`
   - `sc-lint-runtime` creation and runtime-rule migration
   - sprint plan: `docs/sc-lint/sprint-A5.md`
6. `A.6`
   - Rust boundary inventory loader/schema/duplicate handling
   - sprint plan: `docs/sc-lint/sprint-A6.md`
7. `A.7`
   - Rust manifest-policy migration and Python parity window
   - sprint plan: `docs/sc-lint/sprint-A7.md`

These sprint plans are the authoritative implementation breakdown of the
foundation phase once execution begins. This phase document remains the
umbrella scope and sequencing reference.

## Exit Criteria

This phase is complete when:

- canonical `sc-lint` boundaries exist in TOML
- `just lint` is the correct default gate for `sc-lint` development
- the top-level CLI exists and is the preferred user-facing entry point
- generic Python utilities scheduled for extraction are planned or migrated
- shared portability and runtime analyzer families are either migrated or
  explicitly deferred with rationale
- cross-target preflight expectations are documented, including what they can
  and cannot guarantee
- boundary inventory and manifest-policy migration to Rust is staged with
  parity validation

## Release 1 Alignment

Phase `A` is the foundation for release `0.1.x`.

For release `0.1.x`, the repo should be able to demonstrate:

- self-hosting lint discipline
- canonical boundary ownership for current and planned tool surfaces
- a stable CLI plan ready for implementation
- a sequenced migration path rather than ad hoc extraction work

This phase is intentionally a foundation phase. It prepares release `0.1.x`,
rather than claiming that the full release-1 feature set is already shipped.

## Deferred Items

The following stay out of this phase unless explicitly pulled in:

- ATM-specific daemon/test-runtime lints
- forced Rust rewrites of simple Python utilities
- graph-db integration
- generalized view-site polish beyond what is needed for extraction
