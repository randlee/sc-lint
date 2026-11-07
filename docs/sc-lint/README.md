# `sc-lint` Docs

This folder is the home for `sc-lint` design and planning material.

The versioned end-user operator manual is maintained in the repository root at
[`../../docs-bundle/README.md`](../../docs-bundle/README.md) and is discoverable
offline with `sc-lint docs`. It includes the canonical
[`just-setup.md`](../../docs-bundle/just-setup.md) guide and one guide for each
published package.

The current analyzer, consumer-profile, setup, and release-recovery contracts
are documented in that offline bundle; accepted planning decisions remain in
the ADRs and sprint records below.

Contents:

- [`requirements.md`](./requirements.md) — top-level `sc-lint` crate
  requirements summary
- [`architecture.md`](./architecture.md) — top-level `sc-lint` crate
  architecture summary
- [`mvp.md`](./mvp.md) — MVP design for the initial `sc-lint-boundary`
  analyzer and the paired `sc-lint-attributes` plan
- [`roadmap.md`](./roadmap.md) — decisions, rollout sequence, and what stays in
  Python vs what moves to Rust
- [`extraction-plan.md`](./extraction-plan.md) — extraction plan for remaining
  generic lint/view tooling and the Python-to-Rust boundary migration
- [`foundation-phase-plan.md`](../plans/phase-A/foundation-phase-plan.md) — archived
  Phase A foundation execution plan for repo self-hosting, boundaries, CLI
  introduction, and extraction order
- [`phase-B-plan.md`](../plans/phase-B/phase-B-plan.md) — archived Phase B hardening
  and process-tightening plan
- [`phase-C-plan.md`](../plans/phase-C/phase-C-plan.md) — archived Phase C
  interface-versioning, shared-reporting, observability-maintenance, and
  portability plan
- [`phase-D-plan.md`](../plans/phase-D/phase-D-plan.md) — Phase D
  boundary-inventory improvement plan
- [`python-bindings.md`](./python-bindings.md) — the `sc-lint` Python wheel:
  pyo3 surface, consumer venv provisioning, and the PyPI release channel
- [`crate-architecture.md`](./crate-architecture.md) — crate-by-crate role,
  ownership, and phase-plan touchpoint guide
- [`interface-reporting-constraints.md`](./interface-reporting-constraints.md) —
  repo-local constraints for the shared HTML/XHTML/JSON reporting line
- [`version-adoption.md`](./version-adoption.md) — authoritative consumer
  adoption guide for the planned `sc-lint-version` interface-checking line
- [`adoption.md`](./adoption.md) — authoritative guide for installing the
  managed `sc-lint` consumer contract and extending it safely
- [`skill-authoring-constraints.md`](./skill-authoring-constraints.md) —
  repo-local constraints for the `sc-lint-version` adoption skill
- [`minimal-marketplace-constraints.md`](./minimal-marketplace-constraints.md) —
  repo-local constraints for minimal marketplace publication of the adoption
  skill
- [`../../.claude/skills/sc-lint-version-adoption/SKILL.md`](../../.claude/skills/sc-lint-version-adoption/SKILL.md) —
  repo-local Claude Code skill for `sc-lint-version` consumer adoption
- [`../../.claude-plugin/marketplace.json`](../../.claude-plugin/marketplace.json) —
  repo-local marketplace entry advertising the `sc-lint-version` adoption
  skill package
- [`adr/README.md`](./adr/README.md) — ADR index for the current architecture
  decisions
- [`../sc-lint-boundary/requirements.md`](../sc-lint-boundary/requirements.md) —
  `sc-lint-boundary` crate requirements
- [`../sc-lint-boundary/architecture.md`](../sc-lint-boundary/architecture.md) —
  `sc-lint-boundary` crate architecture
- [`../sc-lint-boundary/graph-schema.md`](../sc-lint-boundary/graph-schema.md) —
  current graph/export contract and rule-id inventory
- [`../sc-lint-boundary/boundary-enforcement-model.md`](../sc-lint-boundary/boundary-enforcement-model.md) —
  planned warn/error escalation model for inventory-parity boundary
  enforcement
- [`../sc-lint-boundary/boundary-toml-migration.md`](../sc-lint-boundary/boundary-toml-migration.md) —
  migration plan for moving canonical boundary data from Markdown-embedded
  records to standalone TOML
- [`../sc-lint-portability/requirements.md`](../sc-lint-portability/requirements.md) —
  `sc-lint-portability` crate requirements
- [`../sc-lint-portability/architecture.md`](../sc-lint-portability/architecture.md) —
  `sc-lint-portability` crate architecture
- [`../sc-lint-runtime/requirements.md`](../sc-lint-runtime/requirements.md) —
  `sc-lint-runtime` crate requirements
- [`../sc-lint-runtime/architecture.md`](../sc-lint-runtime/architecture.md) —
  `sc-lint-runtime` crate architecture
- [`../sc-lint-version/requirements.md`](../sc-lint-version/requirements.md) —
  planned `sc-lint-version` crate requirements
- [`../sc-lint-version/architecture.md`](../sc-lint-version/architecture.md) —
  planned `sc-lint-version` crate architecture
- [`sprint-A1a.md`](../plans/phase-A/sprint-A1a.md) — top-level CLI bootstrap and contract
  definition sprint
- [`sprint-A1b.md`](../plans/phase-A/sprint-A1b.md) — top-level config loading and first
  backend integration sprint
- [`sprint-A2.md`](../plans/phase-A/sprint-A2.md) — profiles and `xwin` sprint
- [`sprint-A3.md`](../plans/phase-A/sprint-A3.md) — generic utility extraction sprint
- [`sprint-A4.md`](../plans/phase-A/sprint-A4.md) — portability crate extraction sprint
- [`sprint-A5.md`](../plans/phase-A/sprint-A5.md) — runtime crate extraction sprint
- [`sprint-A6.md`](../plans/phase-A/sprint-A6.md) — Rust boundary inventory loader sprint
- [`sprint-A7.md`](../plans/phase-A/sprint-A7.md) — manifest-policy and parity sprint
- [`sprint-A8.md`](../plans/phase-A/sprint-A8.md) — per-tool user-guide sprint
- [`sprint-B1.md`](../plans/phase-B/sprint-B1.md) — carry-forward lint-gate backlog and
  portability-scope hardening sprint
- [`sprint-B2.md`](../plans/phase-B/sprint-B2.md) — named-caller allowlist enforcement sprint
- [`sprint-B3.md`](../plans/phase-B/sprint-B3.md) — observability boundary-policy ADR sprint
- [`sprint-B4.md`](../plans/phase-B/sprint-B4.md) — QA-process hardening sprint
- [`sprint-B-homebrew.md`](../plans/phase-B/sprint-B-homebrew.md) — full Homebrew toolset
  distribution planning sprint
- [`sprint-C1.md`](../plans/phase-C/sprint-C1.md) — `sc-lint-version` policy and baseline
  planning sprint
- [`sprint-C2.md`](../plans/phase-C/sprint-C2.md) — shared report-template pipeline planning
  sprint
- [`sprint-C3.md`](../plans/phase-C/sprint-C3.md) — hard-fail version gate integration
  planning sprint
- [`sprint-C4.md`](../plans/phase-C/sprint-C4.md) — consumer integration and skill design
  planning sprint
- [`sprint-C5.md`](../plans/phase-C/sprint-C5.md) — minimal marketplace publication planning
  for the adoption skill
- [`sprint-C6.md`](../plans/phase-C/sprint-C6.md) — production path-literal portability
  parity sprint
- [`sprint-C7.md`](../plans/phase-C/sprint-C7.md) — broad environment-variable portability
  sprint
- [`sprint-C8.md`](../plans/phase-C/sprint-C8.md) — shell invocation portability sprint
- [`sprint-C9.md`](../plans/phase-C/sprint-C9.md) — cross-platform `cfg` parity enforcement
  sprint
- [`sprint-C10.md`](../plans/phase-C/sprint-C10.md) — `sc-observability` `1.1.0` adoption
  sprint
- [`sprint-D1.md`](../plans/phase-D/sprint-D1.md) — direct workspace package-edge
  enforcement planning sprint
- [`cli-requirements.md`](./cli-requirements.md) — detailed requirements for
  the top-level `sc-lint` CLI
- [`cli-architecture.md`](./cli-architecture.md) — detailed architecture for
  the top-level `sc-lint` CLI
- [`cli-contract.md`](./cli-contract.md) — implemented top-level success/error
  envelope and backend-to-CLI normalization contract
- [`adr/ADR-012-consumer-adoption-and-just-contract.md`](./adr/ADR-012-consumer-adoption-and-just-contract.md)
  — installed-product consumer commands and generated Just integration
- [`adr/ADR-013-analyzer-shared-support.md`](./adr/ADR-013-analyzer-shared-support.md)
  — shared rule-neutral AST support without analyzer-to-analyzer dependencies
- [`logging.md`](./logging.md) — structured logging design, rollout, and event
  schema for the top-level CLI
- [`../../README.md`](../../README.md) — top-level CLI crate and workspace guide
- [`../../crates/sc-lint-boundary/README.md`](../../crates/sc-lint-boundary/README.md) —
  user guide for `sc-lint lint sc-boundary`
- [`../../crates/sc-lint-portability/README.md`](../../crates/sc-lint-portability/README.md) —
  user guide for `sc-lint lint sc-portability`
- [`../../crates/sc-lint-runtime/README.md`](../../crates/sc-lint-runtime/README.md) —
  user guide for `sc-lint lint sc-runtime`
- [`../../crates/sc-lint-schema/README.md`](../../crates/sc-lint-schema/README.md) —
  shared schema crate guide
- [`../../crates/sc-lint-analyzer-support/README.md`](../../crates/sc-lint-analyzer-support/README.md) —
  shared analyzer support crate guide
- [`../../crates/sc-lint-directives/README.md`](../../crates/sc-lint-directives/README.md) —
  shared directives crate guide
- [`../../crates/sc-lint-attributes/README.md`](../../crates/sc-lint-attributes/README.md) —
  proc-macro attribute crate guide

## Homebrew Distribution

The supported Homebrew install path is:

```bash
brew install randlee/tap/sc-lint
```

That formula installs the CLI and the three analyzer binaries:

- `sc-lint`
- `sc-lint-boundary`
- `sc-lint-portability`
- `sc-lint-runtime`

`randlee/tap/sc-lint-boundary` remains as a legacy formula for boundary-only
callers; it is not the default install path. Release archives and Python
wheels are described in the offline
[installation guide](../../docs-bundle/installation.md).

## Workspace crates

All crates share the workspace version line (0.8.6). The primary product is
`sc-lint`; the other crates are its analyzers and support libraries, and each
crate README points back to `sc-lint`.

- `sc-lint` — the top-level CLI: command parsing, configuration loading,
  output normalization and analyzer dispatch. It owns the machine contract in
  [`cli-contract.md`](./cli-contract.md). Commands: `lint`, `test`, `ci`,
  `check xwin`, `clippy xwin`, `view graph|findings`, `compatibility check`, `setup`,
  `upgrade`, `init`, `docs` and `version`.
- `sc-lint-boundary` — boundary analyzer CLI and library: workspace discovery
  through `cargo_metadata`, module-driven traversal through `syn`, an owner
  graph exported as JSON or Turtle, and the `SCB-CYCLE`, `SCB-BOUNDARY`,
  `SCB-CALLER`, `SCB-DEPENDENCY`, `SCB-FEATURE` and `SCB-MANIFEST` rules.
- `sc-lint-portability` — OS and platform portability rules `PORT-001` to
  `PORT-010`.
- `sc-lint-runtime` — std and Tokio runtime rules `SCB-RUNTIME-001` to
  `SCB-RUNTIME-007`. The product requirement REQ-PRODUCT-004E is Accepted; the detailed
  REQ-RUNTIME-001 ([`../sc-lint-runtime/requirements.md`](../sc-lint-runtime/requirements.md))
  is Pending except the accepted `#![cfg(test)]` marker ruling.
- `sc-lint-directives` — shared parsing of `#[sc_lint(...)]` directives and
  site allowances.
- `sc-lint-attributes` — the no-op `#[sc_lint(...)]` proc-macro attribute; it
  validates arguments and returns the item unchanged.
- `sc-lint-analyzer-support` — rule-neutral AST and source-discovery support
  shared by the analyzers (ADR-013).
- `sc-lint-schema` — shared finding, output-format and report types.

`sc-lint-version` is planned and has no crate yet; see
[`../sc-lint-version/requirements.md`](../sc-lint-version/requirements.md).

## Analyzer rules

`sc-lint lint sc-boundary` (`sc-lint-boundary`):

- `SCB-CYCLE-001` multi-owner architectural cycle; `SCB-CYCLE-002` type/method
  self-loop; `SCB-CYCLE-003` trait-impl self-loop
- `SCB-BOUNDARY-001` to `SCB-BOUNDARY-004` internal-only visibility and
  references, external impls of `forbid_external_impls` traits, and invalid
  boundary inventory records
- `SCB-BOUNDARY-005` source type or `new` constructor wider than the boundary
  record's declared `[implementation]` visibility
- `SCB-CALLER-001` named-caller allowlist
- `SCB-DEPENDENCY-001` to `SCB-DEPENDENCY-004` workspace package edges and
  stale permissions
- `SCB-FEATURE-001` use of another package's features outside
  `allowed_feature_dependencies`
- `SCB-MANIFEST-001` to `SCB-MANIFEST-006` manifest and dependency ownership
  policy

`sc-lint lint sc-portability` (`sc-lint-portability`):

- test code: `PORT-001` Unix-only absolute paths, `PORT-002` unchecked
  `dirs::home_dir()`, `PORT-003` `std::env::set_var()`
- production code: `PORT-004` ungated `std::os::unix` imports, `PORT-005`
  `cfg_attr(not(unix), allow(dead_code))` suppressors, `PORT-006` Unix-only
  path literals, `PORT-007` Windows path literals, `PORT-008` `HOME`/`USER`/`XDG_*`
  lookups, `PORT-009` Unix shell invocations, `PORT-010` `cfg(unix)` items
  without a Windows or portable companion

`sc-lint lint sc-runtime` (`sc-lint-runtime`):

- `SCB-RUNTIME-001` bare `Condvar::wait`; `SCB-RUNTIME-002` discarded
  `wait_timeout*` result; `SCB-RUNTIME-003` `std::thread::sleep` and
  `SCB-RUNTIME-004` `std::thread::sleep_until` in Tokio packages;
  `SCB-RUNTIME-005` `tokio::time::sleep*`; `SCB-RUNTIME-006` `block_on`;
  `SCB-RUNTIME-007` a `std::sync::Mutex` guard held across `.await`

The offline bundle's package guides
([boundary](../../docs-bundle/packages/sc-lint-boundary.md),
[portability](../../docs-bundle/packages/sc-lint-portability.md),
[runtime](../../docs-bundle/packages/sc-lint-runtime.md)) describe each rule,
its default severity and its allowances.

## Lint targets and profiles

`sc-lint lint <target>` accepts the analyzer targets `sc-boundary`,
`sc-portability` and `sc-runtime`, the repository-policy targets
`function-length`, `line-counts` and `identity-literals`, and three profiles:

- `fast` — `cargo fmt --check` plus the Python source checks (version sync,
  manifests, codespell, pytests); no `xwin`
- `ci` — `fast` plus clippy, cargo-deny, cargo-shear and the three analyzers;
  no `xwin`
- `full` — `ci` plus function-length, line-counts and identity-literals, and
  `xwin check` / `xwin clippy` when `cargo xwin` is installed

`sc-lint ci` runs lint plus tests. `sc-lint lint --consumer` instead runs the
consumer steps declared in `sc-lint.toml`; see the offline
[usage guide](../../docs-bundle/using-sc-lint.md).

The Python checks live in the `sc_lint` package at
[`bindings/sc-lint-py/python/sc_lint/`](../../bindings/sc-lint-py/python/sc_lint/)
(for example `run_lint.py`, `lint_line_counts.py`,
`lint_identity_literals.py`, `view_findings.py` and `python_adapter.py`).

Profile orchestration adds no rule-disable flags and does not override backend
rule configuration; rule selection stays with the owning analyzer's
configuration.

## This repository's recipes

The root `Justfile` uses the same consumer contract that adopters get:

- `just setup` — checks the compatible product binary
- `just lint` — runs every `[[tool.sc-lint.lint]]` step in `sc-lint.toml`
  (currently `source-full`, which runs `sc-lint lint full` through
  `python -m sc_lint.run_lint full`); `just lint ci` is accepted and runs the
  same steps
- `just test` — runs every `[[tool.sc-lint.test]]` step (currently
  `source-workspace`: cargo tests, Python tests and the Action tests)

A single analyzer or profile runs directly, for example
`target/debug/sc-lint lint sc-boundary` or
`target/debug/sc-lint lint fast`.

## Boundary sources

Canonical boundary TOML lives under [`boundaries/`](../../boundaries/), one
directory per crate plus `planning.toml`. The same layout is the pattern for
consumers:

```text
boundaries/
  consumer-core/
    mail-store.toml
    identity-registry.toml
  runtime-service/
    server-transport.toml
  planning.toml
```

## Planned work

- `sc-lint-version` and `sc-lint check interfaces` (see
  [`version-adoption.md`](./version-adoption.md))
- Phase D inventory-parity rules `SCB-INVENTORY-001` to `SCB-INVENTORY-003`
  (see [`../sc-lint-boundary/boundary-enforcement-model.md`](../sc-lint-boundary/boundary-enforcement-model.md))

The current plan is [`../project-plan.md`](../project-plan.md); the
decision history is in [`roadmap.md`](./roadmap.md).

## Related architecture decisions

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
- [`./adr/ADR-009-observability-boundary-policy.md`](./adr/ADR-009-observability-boundary-policy.md)
  — accepted observability boundary seams and future direct-link constraints
- [`./adr/ADR-010-portability-scope-and-parity.md`](./adr/ADR-010-portability-scope-and-parity.md)
  — shared portability ownership/parity policy for Windows-path, env, and shell lint expansion
- [`./adr/ADR-013-analyzer-shared-support.md`](./adr/ADR-013-analyzer-shared-support.md)
  — shared rule-neutral analyzer support

Current user guides are the per-crate `README.md` files and the installed
offline guides under `docs-bundle/`. Design and planning documentation remains
under `docs/`.
