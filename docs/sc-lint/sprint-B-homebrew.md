---
id: sprint-B-homebrew
title: Full Toolset Homebrew Distribution
status: planned
branch: feature/phase-B-sprint-plans
worktree: <repo-worktree>/feature/phase-B-sprint-plans
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
- external Homebrew tap checkout rooted at `${HOMEBREW_TAP_DIR}`

## Exact Targets

- `release/publish-artifacts.toml`
- `.github/workflows/release.yml`
- `docs/release-inventory-schema.json`
- `README.md`
- `docs/sc-lint/README.md`
- `${HOMEBREW_TAP_DIR}/Formula/sc-lint.rb`
- `${HOMEBREW_TAP_DIR}/Formula/sc-lint-boundary.rb`

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

- `brew install randlee/tap/sc-lint` succeeds and installs the full toolset
  binaries
- the selected formula strategy is documented with `sc-lint` as the primary
  production formula and any retained `sc-lint-boundary` formula limited to an
  explicit compatibility role
- `release/publish-artifacts.toml` and `.github/workflows/release.yml` define a
  publish path for `sc-lint`, `sc-lint-boundary`, `sc-lint-portability`, and
  `sc-lint-runtime` through the chosen Homebrew formula strategy
- Homebrew automation computes checksums from published release artifacts and
  updates `${HOMEBREW_TAP_DIR}/Formula/sc-lint.rb` deterministically for macOS
  Intel, macOS ARM, and Linux
- formula verification proves the selected Homebrew path exposes
  `sc-lint --version` and the backend binaries promised by the chosen release
  packaging shape
- user docs point at the primary `sc-lint` formula and explain any retained
  compatibility role for `sc-lint-boundary`

## Required Validation

- `python3 scripts/release_artifacts.py validate-manifest --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
- `python3 scripts/release_artifacts.py validate-preflight-checks --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
- `python3 scripts/release_artifacts.py validate-publish-order --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
- `cargo build --workspace`
- `ruby -c "${HOMEBREW_TAP_DIR}/Formula/sc-lint.rb"`
