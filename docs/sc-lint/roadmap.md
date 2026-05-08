# `sc-lint` Roadmap

## Current Decisions

These points are considered settled for the initial spike.

### Crate split

Current implemented crates:

- `sc-lint-directives`
  - shared directive parsing/types
- `sc-lint-boundary`
  - analyzer CLI + library
  - currently shares the workspace `0.1.0` version line
- `sc-lint-attributes`
  - proc-macro attribute crate
  - currently shares the workspace `0.1.0` version line

Planned next crate:

- `sc-lint`
  - top-level CLI crate
  - stable user-facing command surface
  - command parsing, config loading, output normalization, tool dispatch
  - planned lint profiles:
    - `fast`
    - `full`
    - `ci`
  - planned top-level CI-equivalent command:
    - `sc-lint ci`
  - planned Windows preflight commands when `cargo xwin` is installed:
    - `sc-lint check xwin`
    - `sc-lint clippy xwin`

Reason:

- real Rust attributes need a proc-macro crate anyway
- creating it early avoids late packaging churn
- the analyzer crate should not carry proc-macro concerns
- the top-level CLI should coordinate backends rather than forcing backend
  crate cross-dependencies

Current scaffold state:

- `sc-lint-directives`
  - created
  - compile-valid
- `sc-lint-attributes`
  - created
  - compile-valid
  - initial `#[sc_lint(...)]` namespace reserved now
- `sc-lint-boundary`
  - created
  - crate/module/type/impl graph scaffold in place now
  - initial `sc_lint` attribute ingestion in place now
  - first owner-graph cycle rules in place now
  - first boundary enforcement rules in place now
- `sc-lint`
  - planned
  - detailed CLI requirements/architecture defined
  - implementation not started yet

### Analyzer strategy

Use `syn` directly for cycle logic and boundary analysis.

Do **not** use `cargo-modules` as the primary source of truth for semantic
cycle detection.

Reason:

- `cargo-modules` currently reports self-loop noise
- post-processing its textual output is weaker than owning the graph model
- `syn` gives a better base for future boundary rules

### Graph-first design

The analyzer should build an internal code graph first and derive findings from
that graph.

This is intentionally broader than a one-off cycle checker, because the same
graph can later support:

- cycle rules
- visibility/sealed rules
- unsafe ownership rules
- type-coupling analysis
- external graph export

### Attribute philosophy

Attributes are primarily for **declarative boundary intent**, not suppression.

Good early uses:

- internal-only items
- forbidden external implementations
- forbidden external use classes
- boundary roots

Suppression, if it ever becomes necessary, should be:

- rule-specific
- explicit
- auditable

### Python scope

Keep Python for:

- orchestration
- config loading
- report generation
- external tool wrapping
- simple manifest/text checks

Do not grow complicated Rust parsing logic in Python.

## First Deliverable

The first useful deliverable is an internal analyzer that can:

1. discover workspace crates and targets
2. parse source trees with `syn`
3. build a graph for crate/module/type/method ownership
4. export graph JSON
5. classify cycle shapes
6. distinguish self-loop/tool-noise cases from multi-owner architectural cycles

## Immediate Rule Scope

The first sprint cut now includes:

- cycle analysis
- initial boundary enforcement

The analyzer should be able to classify at least:

- `type_method_self_loop`
- `trait_impl_self_loop`
- `multi_owner_architectural_cycle`

The exact names may change, but the rule categories should stay stable enough
to support JSON findings and later config.

Current implementation status:

- implemented:
  - `type_method_self_loop`
  - `trait_impl_self_loop`
  - `multi_owner_architectural_cycle`
  - explicit recursive container/value allowance through
    `boundary.allow("cycle.recursive_value_container")`
  - `internal_only` visibility/reference enforcement
  - `forbid_external_impls` enforcement
- deferred:
  - additional boundary declarations beyond current attribute set

## What Is Explicitly Deferred

- full visibility enforcement
- full sealed-trait enforcement
- unsafe rule enforcement
- dead-code detection
- graph database integration
- full attribute-driven policy expression

## Extraction Path

The extraction step is complete; `sc-lint` now has its own standalone
repository.

The current rollout is:

1. stabilize:
   - CLI contract
   - JSON findings shape
   - graph export shape
   - graph schema versioning
   - attribute namespace
2. define and enforce canonical repo boundaries
3. introduce the top-level CLI
4. migrate remaining generic tooling
5. prepare the standalone repo for crates.io publication

## Near-Term Integration Expectation

The analyzer is expected to plug into the existing lint surface rather than
replace it wholesale.

Likely future integration:

- Python runner invokes `sc-lint-boundary`
- JSON findings are logged and rendered through the existing lint tooling
- `just lint modules` or a replacement rule target eventually uses the analyzer
  instead of raw `cargo-modules --acyclic`

Current integration state:

- `just lint sc-boundary`
  - exists now as a named target
  - is part of default `just lint` for this repo
- `just lint sc-portability`
  - exists now as a named target
  - is part of default `just lint` for this repo

Current planned local/CI profile split:

- `fast`
  - low-latency local developer gate
  - may include `xwin check` when available
- `full`
  - stronger local pre-push gate
  - may include `xwin check` and `xwin clippy` when available
- `ci`
  - lint-only CI-parity profile
  - intentionally excludes `xwin` because real Windows CI remains
    authoritative
- top-level `ci`
  - lint plus tests

## Default Rule Policy

`sc-lint-boundary` ships with an embedded default rule config at:

- `crates/sc-lint-boundary/config/defaults.toml`

It currently carries the built-in `trait_self_loop` policy through:

- `ignored_trait_paths`
- `ignored_trait_names`

This is the default-install extension point for common non-architectural trait
families such as comparison, hashing, conversion, and serde traits.

## Next Planning Items

The next planned boundary-enforcement work after the current implementation
branch merges is:

1. boundary definition migration from Markdown parsing to TOML
2. inventory-parity warn/error enforcement on top of the TOML-backed boundary
   model

These are documented in:

- [`boundary-enforcement-model.md`](./boundary-enforcement-model.md)
- [`boundary-toml-migration.md`](./boundary-toml-migration.md)

Current direction for both items:

- TOML dual-loader support and canonical schema should land before
  inventory-parity warn/error enforcement
- inventory-parity checks should compare structured boundary data against the
  code graph
- planned future-sprint gaps may warn temporarily, but must auto-escalate when
  overdue
- TOML should become the canonical source for new boundary features as soon as
  TOML loading exists

## Consumer-Proven Rule Promotion

The current plan explicitly treats some rule families as consumer-proven first
and productized second.

Reusable analyzer families first proven on `atm-core` and planned for
standalone `sc-lint`:

- `PORT-004`
- `PORT-005`
- `SCB-RUNTIME-001`
- `SCB-RUNTIME-002`

Consumer-local policy families that stay out of `sc-lint` unless extracted as
configurable framework:

- duplicate semantic string-literal policy
- fixed-sleep test-hygiene policy
- triage Turtle consistency policy

Related ADR:

- [`./adr/ADR-004-structured-boundary-definitions.md`](./adr/ADR-004-structured-boundary-definitions.md)

## Release 1 Direction

Release `0.1.x` should establish:

- stable repo-local lint gating
- canonical TOML boundaries for current and planned tool surfaces
- a documented top-level CLI contract ready for implementation
- a staged extraction and migration path for remaining generic tooling

This is the release-1 direction, not a claim that every release-1 target is
already implemented today.
