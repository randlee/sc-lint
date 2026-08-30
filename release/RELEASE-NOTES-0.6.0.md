# sc-lint v0.6.0 Release Notes

## Release

- Version: `0.6.0`
- Date: 2026-08-30
- Release owner: sc-lint maintainers
- Approval: pending release publication approval

## Summary

Phase G delivers the versioned adoption kit and skill so any Rust repository
can adopt sc-lint's consumer contract with one drift-detectable install, backed
by a self-contained release (Python wheel runtime + archive binaries).

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

## Migration Notes

- Consumer repositories should use `sc-lint init --just` and keep their public
  task surface to `just setup`, `just lint`, `just test`, and `just upgrade`.
- Set `[tool.sc-lint].minimum_version = "0.6.0"` in `sc-lint.toml` when adopting
  the Phase G consumer contract.
- Source checkout contributors can continue using `cargo run` for development;
  consumer-facing behavior is owned by the installed `sc-lint` product.

## Validation

- Phase G CI passed (Test, Just lint, Release smoke, Adoption kit) Just lint and test jobs on Ubuntu, macOS, and Windows.
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
