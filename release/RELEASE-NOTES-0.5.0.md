# sc-lint v0.5.0 Release Notes

## Release

- Version: `0.5.0`
- Date: 2026-08-12
- Release owner: sc-lint maintainers
- Approval: pending release publication approval

## Summary

Phase E makes sc-lint a complete consumer-delivery product: repositories can
declare a minimum compatible version, install or upgrade a verified release,
use a small canonical `just` interface, discover offline documentation, and
run the same contract locally and in CI.

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

- Compatibility contract and recovery: consumer `sc-lint.toml` files declare a
  SemVer minimum, with structured version probes and compatibility diagnostics.
- Verified setup and upgrade: releases are checksum-verified, activated
  atomically, rolled back on failed verification, and exposed through
  idempotent consumer setup and upgrade commands.
- Consumer CLI and Just integration: `sc-lint init --just` provides the
  product-owned bootstrap plus canonical `just setup`, `just lint`,
  `just test`, and `just upgrade` recipes.
- Distributed documentation and release delivery: installed releases include
  offline guides, package documentation, help discovery, and Homebrew support.
- Reusable GitHub Action: verified release artifacts can be installed and run
  in consumer CI workflows.
- Dogfooded consumer contract: the root repository supplies cross-platform
  release-binary fixtures covering setup, lint, test, documentation discovery,
  and safe upgrades.
- New first-time-published crate: `sc-lint-analyzer-support` owns shared
  analyzer source scanning and report rendering extracted during the Phase E
  `ARCH-001` deduplication work.

## Migration Notes

- Consumer repositories should use `sc-lint init --just` and keep their public
  task surface to `just setup`, `just lint`, `just test`, and `just upgrade`.
- Set `[tool.sc-lint].minimum_version = "0.5.0"` in `sc-lint.toml` when adopting
  the Phase E consumer contract.
- Source checkout contributors can continue using `cargo run` for development;
  consumer-facing behavior is owned by the installed `sc-lint` product.

## Validation

- Phase E CI passed Just lint and test jobs on Ubuntu, macOS, and Windows.
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
