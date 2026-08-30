---
title: Phase E Plan — Consumer Adoption And Distributed Documentation
status: implemented
branch: feature/plan-sc-lint-usability
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-usability
target: develop
---

# Phase E Plan — Consumer Adoption And Distributed Documentation

## Goal

Make `sc-lint` adoptable without consumers learning the source repository's
Cargo topology, copied Python wrappers, or release implementation details.

A consumer repository must be able to standardize on these commands:

```text
just setup
just lint
just test
just upgrade
```

`just lint` runs every required lint and `just test` runs every required test.
Both commands first verify that the system installation of `sc-lint` satisfies
the repository's declared minimum version. The same installed product ships
its documentation and exposes it from its help menu.

## Baseline

- current planning location: `docs/phase-E/`
- intended long-term location: `docs/plans/phase-E/`; relocation is explicitly
  out of scope for this phase and will be performed by the separate plans-tree
  migration PR
- current primary release formula: `randlee/tap/sc-lint`
- current release archives contain four binaries but no distributable
  documentation bundle
- current `Justfile` and `.just/` scripts mix source-repository orchestration
  with consumer-facing behavior
- current `sc-lint --version`/`sc-lint version` surface already establishes a
  version concept, but no documented machine contract is available for a
  bootstrapper to compare against a repository requirement

## Relationship To Parked PR #87

PR #87 is not a Phase E dependency or an acceptance oracle. Its cargo-deny CI
repair proceeds separately as PR #92. Its source-versus-consumer Python-runner
rework is parked, has no committed timeline, and is superseded by E.3's
explicit product-owned bootstrap and installed `sc-lint` command contract.
Phase E must neither merge nor copy that runner rework.

The historical failure is retained only as an in-repo consumer-fixture case:
an absent installed `sc-lint` must produce the E.1 structured recovery result,
not an executable-launch exception. E.7 defines and verifies that case without
depending on any external pull request.

## Product Decisions Locked By This Phase

### Consumer configuration

The canonical repository configuration is `sc-lint.toml` and includes an
explicit compatibility floor:

```toml
[tool.sc-lint]
minimum_version = "0.4.1"
```

The floor means `installed_version >= minimum_version` under SemVer ordering.
An absent, malformed, prerelease-incompatible, or older installation is not
silently accepted.

### Command ownership

- `just` is the short, memorable developer and agent interface.
- `sc-lint` is the installed product that owns consumer lint/test/setup/upgrade
  behavior, diagnostics, and documentation discovery.
- `cargo run -p sc-lint -- ...` is source-checkout development machinery only.
- consumers never choose a Cargo package or invoke an analyzer sibling binary
  directly.

### Documentation ownership

The distributed documentation bundle is versioned with the release and is
installed beside the product, never copied over a consuming repository's own
`README.md`. The bundle contains an overview `README.md`, one guide for every
published `sc-lint-*` package, and the canonical `just` setup guide.

The overview documentation is not a short release note. It is the operator
manual for installation, first use, configuration, everyday lint/test use,
CI, upgrading, troubleshooting, and recommended repository/agent practices.
The `just` guide is the canonical model consumers copy or generate.

### Error contract

Following `RBP-001`, every installation/version/documentation failure must
carry a stable machine-readable code, the observed and required values when
available, a recovery action, and a documentation reference. A missing binary
must never become an uncaught Python traceback.

### Requirements and architecture records

Phase E changes product behavior rather than only implementation detail. E.1
must implement `REQ-PRODUCT-019` for version compatibility and structured
recovery. E.2 must implement `REQ-PRODUCT-020` for safe installation and
upgrade. E.3 must add an ADR for the consumer adoption/Just contract and
update the product and CLI architecture documents. E.4/E.5 must implement
`REQ-PRODUCT-021` for installed documentation/discovery and release delivery.
E.6 must implement `REQ-PRODUCT-022` for the reusable GitHub Action. No
implementation sprint may close before its assigned requirements and
ADR/documentation updates land.

| Requirement | Authorized scope | Owning sprint(s) | Closure evidence |
| --- | --- | --- | --- |
| `REQ-PRODUCT-019` | minimum-version configuration, compatibility preflight, and complete consumer Just entry points | E.1, E.3, E.7 | CLI contract, ADR-012, generated-template and root-model fixtures |
| `REQ-PRODUCT-020` | checksum-verified installation, atomic replacement, and recoverable upgrade | E.2, E.7 | installer rollback/version tests and upgrade fixture lane |
| `REQ-PRODUCT-021` | offline documentation bundle, help discovery, release archive, and Homebrew layout | E.4, E.5, E.7 | bundle manifest/link validation and staged archive/Homebrew tests |
| `REQ-PRODUCT-022` | versioned reusable GitHub Action over verified release artifacts | E.6, E.7 | Action requirements, metadata validation, and cross-platform Action fixtures |

## Phase Entry Criteria

- Phase D's accepted `develop` baseline remains green.
- the release manifest and Homebrew renderer remain the source of truth for
  the shipped binary set.
- the dedicated planning worktree is used; no change is made in the primary
  `develop` checkout.

## Sprint Sequence

### E.1 Compatibility Contract And Version Preflight

Define the `sc-lint.toml` minimum-version schema, stable version probe, typed
compatibility evaluation, and recoverable diagnostics.

Depends on: Phase entry criteria.

Unblocks: E.2, E.4, E.5.

### E.2 Installation And Upgrade Engine

Provide the idempotent bootstrap, installation, and upgrade engine that lets a
consumer recover a missing/old system installation without hand-writing tool
logic.

Depends on: E.1.

Unblocks: E.3, E.4, E.5.

### E.3 Consumer CLI And Canonical Just Integration

Move consumer orchestration behind installed `sc-lint` commands and generate
thin `Justfile` integration that performs the E.1 preflight on every public
recipe.

Depends on: E.1, E.2.

Unblocks: E.4, E.5, E.7.

### E.4 Documentation Bundle, Operator Manual, And Help Discovery

Create the versioned documentation bundle, extensive operator manual, package
guides, canonical Just guide, and CLI help/documentation discovery contract.

Depends on: E.1, E.2, E.3.

Unblocks: E.5, E.6.

### E.5 Release Distribution And Documentation Package

Package the already-defined product and documentation bundle into deterministic
release archives and the Homebrew formula.

Depends on: E.3, E.4.

Unblocks: E.6, E.7.

### E.6 GitHub Action Consumer Delivery

Publish the reusable GitHub Action over the released E.5 artifacts and prove
that its setup/lint/test interface enforces the same compatibility contract.

Depends on: E.3, E.5.

Unblocks: E.7.

### E.7 Dogfooding, Consumer Fixtures, And Cross-Platform Acceptance

Make this repository the reference consumer, and prove fresh install,
too-old-version remediation, lint/test, documentation discovery, and upgrade
flows on Linux, macOS, and Windows.

Depends on: E.1 through E.6.

## Scope Rules

Phase E may:

- add a stable consumer configuration schema and explicit version errors
- add installer, upgrade, init, documentation, and GitHub Action surfaces
- package static documentation in release archives and Homebrew installations
- replace copied consumer orchestration with thin generated integration
- add disposable consumer fixtures and cross-platform release-binary tests

Phase E must not:

- overwrite a consumer repository's `README.md`
- require consumers to install every source-repository development tool
- make `just lint` or `just test` partial/advisory commands
- infer source versus consumer mode from directory names or Cargo package
  presence
- weaken source-repository formatting, Clippy, test, release, or package gates
- make network installation happen without an explicit `just setup` or
  documented preflight policy and transparent output

## Required Validation Lanes

### Lane A: Source-Checkout Maintenance

`just lint` and `just test` must cover this repository's complete required
lint/test surface, including Rust source quality gates and tests for the new
consumer contracts.

### Lane B: Fresh Consumer

A disposable Rust workspace receives generated configuration and Just
integration, runs `just setup`, then succeeds with `just lint` and `just test`
without knowing any `sc-lint-*` Cargo package name.

### Lane C: Outdated Or Missing Installation

The fixture proves that an absent or lower-than-minimum installed binary is
identified before lint/test execution, upgraded by the supported setup path or
fails with structured recovery guidance when installation is deliberately
disabled.

### Lane D: Packaged Distribution

Release archives, Homebrew, and the GitHub Action expose the same binary
version, documentation bundle, and help/documentation paths on each supported
platform. Following rule `b-mr7cp6x0-ipdnhs`, Linux, macOS, and Windows are
independent required lanes.

## Phase Exit Criteria

- a consumer needs only documented config plus `just` recipes to adopt
  `sc-lint`
- `just lint` and `just test` preflight the required system installation on
  every invocation and run their entire respective suites
- no consumer-facing command relies on copied `.just/*.py` implementation
  scripts, source-tree detection, or raw `cargo run -p <analyzer>` dispatch
- the installed release includes `README.md`, one package guide per published
  package, and a `just` setup guide; `sc-lint help` reaches them
- setup, upgrade, and CI use the same version-resolution semantics
- the current repository passes the same public consumer contract it documents
- all four validation lanes pass on the supported operating-system matrix

## Immediate Planning Outputs

- `docs/phase-E/phase-E-plan.md`
- `docs/phase-E/sprint-E1.md`
- `docs/phase-E/sprint-E2.md`
- `docs/phase-E/sprint-E3.md`
- `docs/phase-E/sprint-E4.md`
- `docs/phase-E/sprint-E5.md`
- `docs/phase-E/sprint-E6.md`
- `docs/phase-E/sprint-E7.md`
