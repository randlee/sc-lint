# sc-lint v0.6.0 Release Notes

## Release

- Version: `0.6.0`
- Date: 2026-09-20
- Release owner: sc-lint maintainers
- Approval: pending release publication approval

## Summary

Phase G delivers the versioned adoption kit and skill so any Rust repository
can adopt sc-lint's consumer contract with one drift-detectable install, backed
by a self-contained release (Python wheel runtime + archive binaries).

The release also includes boundary-model fixes for reference implementation
owners, trait-method identity, structured inventory validation, and the
`sc-lint-boundary` graph schema `0.2.0`.

## Included Crates

<!-- Generated from release/publish-artifacts.toml with list-artifacts --publishable-only. -->

- `sc-lint-directives`
- `sc-lint-schema`
- `sc-lint-analyzer-support`
- `sc-lint-attributes`
- `sc-lint-boundary`
- `sc-lint-portability`
- `sc-lint-runtime`
- `sc-lint`

## Major Changes

- Adoption kit (`packages/sc-lint-adoption`): `install.py` installs/validates
  the consumer end state, `--dry-run` reports drift (exit 1) and user-owned
  conflicts (exit 2), persists managed `.sc-lint/install.json`, and ships the
  reusable `setup-sc-lint` GitHub Action.
- Adoption skill and agent: `sc-lint-adoption` plugin (skill, `sc-lint-adopter`
  agent, evals, marketplace entry) and `docs/sc-lint/adoption.md`.
- Python wheel runtime: helper scripts ship as the `sc-lint` PyPI distribution;
  bootstrap provisions `.sc-lint/venv`.
- Self-contained release: `full`/`ci` lint profiles use only archive binaries
  or wheel helpers; `sc-lint version --json` reports `self_contained`;
  `release-smoke` CI job on every OS.
- Bootstrap hardening: `setup --check/--dry-run` never fetch a release; flags
  forwarded to the managed binary; kit/repo bootstrap copies pinned LF.
- Legacy `.just/lint-config.toml` fallback removed; `sc-lint.toml` is the only
  repo configuration.
- Reference implementation owners receive distinct graph identities (#159).
- Trait methods are distinct from inherent methods, and qualified forwarding
  edges retain their trait-implementation identity (#160, #164).
- Boundary graph exports use `schema_version` `0.2.0` instead of `0.1.0`
  (`crates/sc-lint-boundary/src/lib.rs:42`, #163).
- Structured boundary inventory validation rejects malformed forbidden edges,
  requires `boundaries/planning.toml`, checks `owner_crate_path`, preserves
  edge parse causes, and keeps the Python validator aligned with Rust
  (#115, #168, #171, #173, #175–#178).

## Migration Notes

- Consumer repositories should use `sc-lint init --just` and keep their public
  task surface to `just setup`, `just lint`, `just test`, and `just upgrade`.
- Set `[tool.sc-lint].minimum_version = "0.6.0"` in `sc-lint.toml` when adopting
  the Phase G consumer contract.
- Source checkout contributors can continue using `cargo run` for development;
  consumer-facing behavior is owned by the installed `sc-lint` product.
- For a committed graph or findings export with schema `0.1.0`, discard or
  migrate cached records and stored `node_ids`/edge endpoints; regenerate JSON
  or Turtle with `sc-lint-boundary export-graph --root <repo> --format json`
  (or `turtle`) and rebaseline consumers against `0.2.0`. No legacy ID mapping
  is emitted; use implementation nodes and `contains`/`declares`/`targets`
  edges rather than inferring trait ownership from method-name suffixes
  (`docs/sc-lint-boundary/graph-schema.md:18-43`, #163).

## Validation

- `develop` head is `e2477c38857537c8bcd34cf3873ee9726ef8d12a`.
- Latest `develop` CI run `35535212640` completed successfully (Test, Just
  lint, Release smoke, and Adoption kit).
- Release publication order and package preflight are driven by
  `release/publish-artifacts.toml`.
- The included-crates list above was generated from that same publish manifest.

## Packaging / Publication Notes

- `sc-lint-analyzer-support` is a first-time crates.io publication; its
  manifest entry waits 15 seconds after publish to allow index propagation
  before dependent crates are published.
- Release archives include the consumer bootstrap and offline documentation
  bundle; Homebrew and the GitHub Action consume verified release artifacts.

## Follow-Up Items

- Publish the GitHub release body from this completed note after release
  workflow verification succeeds.

## Test Infrastructure

- Installer fixture tests use a hard-linked native CLI and child-process probe
  writes so the test process does not hold a write descriptor for an executed
  file (#180, `docs/plans/release-0.6.0/lint-spx.11-installer-exec-fixture-race.md`).
