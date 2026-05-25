---
id: sprint-B-homebrew
title: Full Toolset Homebrew Distribution
status: planned
branch: feature/phase-B-sprint-plans
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/phase-B-sprint-plans
target: develop
---

# Sprint B Homebrew — Full Toolset Homebrew Distribution

## Goal

- make `brew install randlee/tap/sc-lint` the primary supported Homebrew entry
  point
- extend Homebrew distribution from `sc-lint-boundary` only to the full
  released `sc-lint` toolset
- close issue `#30` with one production-ready release and tap update path for
  macOS Intel, macOS ARM, and Linux

## Hard Dependencies

- [docs/sc-lint/phase-B-plan.md](./phase-B-plan.md)
- [release/publish-artifacts.toml](../../release/publish-artifacts.toml)
- [.github/workflows/release.yml](../../.github/workflows/release.yml)
- [docs/release-inventory-schema.json](../release-inventory-schema.json)
- `/Users/randlee/Documents/github/homebrew-tap/Formula/sc-lint-boundary.rb`

## Exact Targets

- `release/publish-artifacts.toml`
- `.github/workflows/release.yml`
- `docs/release-inventory-schema.json`
- `README.md`
- `docs/sc-lint/README.md`
- `/Users/randlee/Documents/github/homebrew-tap/Formula/sc-lint.rb`
- `/Users/randlee/Documents/github/homebrew-tap/Formula/sc-lint-boundary.rb`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- Homebrew distribution ships one primary `sc-lint` install path that gives the
  user the full released toolset needed for normal repo use
- release manifest and GitHub release artifacts cover the binaries required for
  that full toolset: `sc-lint`, `sc-lint-boundary`, `sc-lint-portability`, and
  `sc-lint-runtime`
- one formula strategy is selected and documented: a unified `sc-lint` formula
  is the default production path, while any direct backend formula remains
  optional compatibility surface only if the release automation keeps it in
  sync
- release workflow updates the Homebrew tap deterministically for macOS Intel,
  macOS ARM, and Linux with per-artifact checksums taken from published release
  tarballs
- operator docs clearly state the supported `brew` install command, the
  installed binaries, and the expected relationship between the top-level
  formula and any optional backend-specific formula

## Required Work

- extend the publish manifest from boundary-only release binaries to the full
  supported toolset
- decide and record the tap packaging shape before implementation branches:
  prefer a unified `sc-lint` formula that installs all supported binaries from
  one release artifact set
- update the release workflow to compute checksums for every shipped Homebrew
  tarball and rewrite the tap formula deterministically
- define the release-artifact naming and packaging contract so the formula can
  install the same binary set across macOS Intel, macOS ARM, and Linux without
  per-platform drift
- keep `sc-lint-boundary` directly installable only if it can remain generated
  from the same authoritative release metadata as `sc-lint`
- add formula-level verification that proves the installed tap path exposes
  `sc-lint --version` and the backend binaries expected from the selected
  formula strategy
- update user docs so Homebrew guidance points at the primary `sc-lint` formula
  instead of the boundary-only stopgap

## Explicit Code Samples

If the sprint introduces or changes important traits, features, enums, protocol
types, boundary contracts, or execution seams, this section must include
explicit code samples or signatures showing the intended end state.

```toml
[[release_binaries]]
name = "sc-lint"

[[release_binaries]]
name = "sc-lint-boundary"

[[release_binaries]]
name = "sc-lint-portability"

[[release_binaries]]
name = "sc-lint-runtime"
```

```ruby
class ScLint < Formula
  desc "Top-level sc-lint CLI and analyzer toolset for Rust workspaces"

  def install
    bin.install "sc-lint"
    bin.install "sc-lint-boundary"
    bin.install "sc-lint-portability"
    bin.install "sc-lint-runtime"
  end

  test do
    system "#{bin}/sc-lint", "--version"
    system "#{bin}/sc-lint-boundary", "--version"
  end
end
```

## This Sprint Does Not Close

- Winget or other non-Homebrew package-manager parity
- Linux ARM Homebrew support if no corresponding release artifact exists yet
- broad release-process redesign unrelated to the Homebrew full-toolset path

## Acceptance Criteria

- `docs/sc-lint/sprint-B-homebrew.md` remains unnumbered in filename and
  headers while still acting as the authoritative plan for the sprint
- the sprint chooses and documents one production-ready Homebrew formula
  strategy, with `brew install randlee/tap/sc-lint` as the primary install path
- release metadata and workflow updates are sufficient to publish and verify
  `sc-lint`, `sc-lint-boundary`, `sc-lint-portability`, and `sc-lint-runtime`
  through the chosen Homebrew path
- Homebrew automation covers macOS Intel, macOS ARM, and Linux with
  deterministic checksum generation from published release artifacts
- user docs name the installed binaries and no longer describe
  `sc-lint-boundary` as the only supported Homebrew entrypoint

## Required Validation

- `python3 scripts/release_artifacts.py validate-manifest --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
- `python3 scripts/release_artifacts.py validate-preflight-checks --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
- `python3 scripts/release_artifacts.py validate-publish-order --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
- `cargo build --workspace`
- `ruby -c /Users/randlee/Documents/github/homebrew-tap/Formula/sc-lint.rb`
