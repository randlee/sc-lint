# `sc-lint-version` Consumer Adoption

This document is the authoritative adoption guide for consuming repositories
that want to use the planned `sc-lint-version` interface-checking line.

It defines what the consuming repo must provide so `sc-lint-version` can:

- generate canonical interface artifacts
- produce shared HTML/XHTML/JSON reports
- hard-fail on breaking interface drift

## Adoption Goal

A consuming repo should be able to run one planned interface-check flow:

```bash
sc-lint --json check interfaces
```

without inventing a repo-specific contract model for each interface family.

The consuming repo owns test fixtures, simulator inputs, and baseline storage
for its own interfaces. `sc-lint-version` owns the shared artifact model,
family verdicts, and hard-fail semantics.

## Common Consumer Responsibilities

Every consuming repo must provide:

- one explicit decision about which interface families are in scope under
  `[version.families.<family>]`
- baseline storage for each enabled family
- deterministic fixtures or baseline extraction inputs
- one stable output location for generated artifacts and published reports
- repo-local normalization only when unstable values cannot be removed from the
  canonical artifact layer itself

The consuming repo must not:

- maintain hand-written HTML as the authoritative interface record
- bypass canonical artifacts with family-specific one-off diff logic
- invent a second contract schema for the same interface family

## Rust Public API Responsibilities

For the `rust-public-api` family, the consuming repo must provide:

- the set of published crates whose public APIs are in scope
- the baseline source for comparison:
  - latest published release
  - specific published version
  - specific git revision
  - explicit baseline artifact set
- any crate-selection configuration needed to exclude private or unpublished
  crates from the public interface set

The consuming repo does not need to build its own semver engine. The planned
Rust API comparison path is the shared `cargo-semver-checks`-based flow owned
by `sc-lint-version`.

## CLI Interface Responsibilities

For the `cli` family, the consuming repo must provide:

- the command list or command families that define the stable contract surface
- stable CLI fixtures needed to exercise those commands
- the baseline artifact produced by:

```bash
sc-lint --json check interfaces --family cli --write-baseline <path>
```

- normalization hooks only for values that cannot be made stable in the
  canonical artifact generator

Consuming repos should reuse existing CLI testability surfaces where
available. If the CLI already has command simulators, golden JSON fixtures,
stateful fake backends, or existing machine-contract tests, those are the
preferred adoption seam.

Consuming repos should not write a second standalone CLI exerciser when the
repo already has testable command execution helpers.

### CLI Harness Responsibilities

The consuming repo CLI harness is responsible for:

- executing the planned stable commands against representative fixtures
- producing canonical interface artifacts from real command behavior
- isolating environment-sensitive values where possible before normalization
- reusing existing simulator or fake-backend seams instead of building a
  second command path only for versioning

### CLI Fixture Responsibilities

CLI fixtures should provide:

- representative command inputs
- representative success paths
- representative stable error-code paths
- any required workspace or config state needed to exercise the contract

## RPC/Socket Responsibilities

For the `rpc-socket` family, the consuming repo must provide:

- the canonical set of transport operations/messages in scope
- transcript fixtures or simulator inputs for the stable transport contract
- baseline artifacts for the generated interface-control-document surface

If the repo already has stateful simulators, transcript replay, or protocol
fixtures, those are the preferred adoption seam.

The consuming repo should not create an artificial second wire protocol only
for version checks.

### RPC/Socket Simulator Responsibilities

When the repo has transport interfaces, the consuming repo should provide:

- existing simulators when available
- otherwise, transcript fixtures covering the stable request/response surface
- negative-path coverage for stable transport error semantics when those
  semantics are part of the published contract

## Normalization Responsibilities

Normalization hooks are allowed only when unstable values cannot be removed at
the canonical artifact layer.

Examples of acceptable normalization:

- timestamps generated outside the canonical report model
- ephemeral temp paths
- generated ids that are intentionally not part of the stable contract

Examples of unacceptable normalization:

- deleting required contract fields to avoid drift
- renaming stable command identifiers
- hiding breaking field removals

The default expectation is that normalization stays thin. If the consuming repo
needs heavy normalization to make one family usable, that is a signal that the
canonical artifact generator should be improved instead.

## Minimal Adoption Shape

At minimum, a consuming repo should end up with:

```toml
[version.families.rust-public-api]
baseline = "crates.io:0.2.0"

[version.families.cli]
baseline_artifact = "artifacts/baselines/cli-v0.2.0.json"
```

and shared reporting template selection under:

```toml
[reporting.templates."public-api"]
source = "sc-compose:sc-lint-public-api-report"

[reporting.templates.cli]
source = "sc-compose:sc-lint-cli-report"
```

## Adoption Outcome

The intended end state for a consuming repo is:

- stable interface families selected through one config surface
- canonical machine-readable artifacts for each enabled family
- shared HTML/XHTML/JSON publication through the reusable reporting line
- hard-fail checks driven by those same canonical artifacts

## Governing Documents

- [../sc-lint-version/requirements.md](../sc-lint-version/requirements.md)
  Governing requirements:
  `REQ-VERSION-022A`, `REQ-VERSION-022AA`.
- [./skill-authoring-constraints.md](./skill-authoring-constraints.md)
  Governing adoption-skill constraints for scope, structure, and authoring
  responsibilities.
