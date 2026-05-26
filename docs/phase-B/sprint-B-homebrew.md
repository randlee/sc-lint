---
id: sprint-B-homebrew
title: Full Toolset Homebrew Distribution
status: completed
branch: feature/sprint-B-homebrew
worktree: <repo-worktree>/feature/sprint-B-homebrew
target: integration/phase-B
---

# Sprint B Homebrew — Full Toolset Homebrew Distribution

## Goal

- make `brew install randlee/tap/sc-lint` the primary supported Homebrew entry
  point
- bootstrap a new primary `sc-lint.rb` formula through the existing
  `update-homebrew` pipeline so one install path ships the full released
  `sc-lint` toolset
- define a production-ready plan for issue `#30`, covering the full toolset
  release and tap update path for macOS Intel, macOS ARM, and Linux

## Hard Dependencies

- [docs/phase-B/phase-B-plan.md](./phase-B-plan.md)
- [docs/requirements.md](../requirements.md)
- [docs/architecture.md](../architecture.md)
- [release/publish-artifacts.toml](../../release/publish-artifacts.toml)
- [.github/workflows/release.yml](../../.github/workflows/release.yml)
- [docs/release-inventory-schema.json](../release-inventory-schema.json)
- the existing `update-homebrew` workflow checkout rooted at `homebrew-tap/`

## Exact Targets

Paths under `homebrew-tap/` are relative to the workflow's secondary tap
checkout root, not to this repository root. Use `${HOMEBREW_TAP_DIR}` for
local validation and the workflow checkout path `homebrew-tap/` in CI.

- `release/publish-artifacts.toml`
- `.github/workflows/release.yml`
- `docs/release-inventory-schema.json`
- `scripts/release_artifacts.py`
- `README.md`
- `docs/sc-lint/README.md`
- `homebrew-tap/Formula/sc-lint.rb`
- `homebrew-tap/Formula/sc-lint-boundary.rb`

## Deliverables

Every listed deliverable is expected to land at a production-ready level for
the scope this sprint claims. If that cannot be done cleanly in one sprint, the
sprint must be split before implementation begins. No deliverable may be
silently dropped or partially deferred.

- Homebrew distribution ships one primary `sc-lint` install path that gives the
  user the full released toolset needed for normal repo use
- the sprint includes the tap bootstrap step that creates
  `homebrew-tap/Formula/sc-lint.rb`, because the current tap surface only ships
  `sc-lint-boundary.rb`
- release manifest and GitHub release artifacts cover the binaries required for
  that full toolset: `sc-lint`, `sc-lint-boundary`, `sc-lint-portability`, and
  `sc-lint-runtime`
- `release/publish-artifacts.toml` expands the existing schema-version-1
  `[[crates]]` and `[[release_binaries]]` tables to the full multi-crate,
  multi-binary toolset without requiring a schema-version bump
- `scripts/release_artifacts.py` is reviewed and updated only if needed so the
  existing validate-manifest, validate-preflight-checks, validate-publish-order,
  list-release-binaries, and cargo-build-bin-args paths handle the expanded
  multi-binary manifest shape
- `sc-lint.rb` becomes the primary supported install path; the sprint makes the
  disposition of `sc-lint-boundary.rb` explicit by either retiring it from the
  tap or retaining it only as a documented legacy compatibility surface
- release workflow updates the Homebrew tap deterministically for macOS Intel,
  macOS ARM, and Linux with per-artifact checksums taken from published release
  tarballs
- operator docs clearly state the supported `brew` install command, the
  installed binaries, and that the backend tools are included by the top-level
  `sc-lint` formula rather than installed via separate formulas

## Implementation Notes

- `release/publish-artifacts.toml` now keeps the full publishable workspace in
  one schema-version-1 manifest and declares the shipped release binaries in
  top-level-toolset order with `sc-lint` first
- `scripts/release_artifacts.py` owns the Homebrew formula renderer so the
  workflow and local validation path share one deterministic generator
- the release archives for Homebrew are packaged as `sc-lint_<version>_<target>`
  bundles that contain `sc-lint`, `sc-lint-boundary`, `sc-lint-portability`,
  and `sc-lint-runtime`
- `homebrew-tap/Formula/sc-lint-boundary.rb` is retained only as a documented
  legacy compatibility surface; normal users should install
  `randlee/tap/sc-lint`

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
    system "#{bin}/sc-lint-portability", "--version"
    system "#{bin}/sc-lint-runtime", "--version"
  end
end
```

```toml
[[crates]]
artifact = "sc-lint"
package = "sc-lint"

[[crates]]
artifact = "sc-lint-portability"
package = "sc-lint-portability"

[[crates]]
artifact = "sc-lint-runtime"
package = "sc-lint-runtime"
```

The `[[crates]]` sample above is intentionally partial. Real manifest entries
must include the full schema-version-1 field set already required by
`scripts/release_artifacts.py` and demonstrated in
`release/publish-artifacts.toml`:
`cargo_toml`, `required`, `publish`, `publish_order`, `preflight_check`,
`wait_after_publish_seconds`, and `verify_install`.

## This Sprint Does Not Close

- Winget or other non-Homebrew package-manager parity
- Linux ARM Homebrew support if no corresponding release artifact exists yet
- broad release-process redesign unrelated to the Homebrew full-toolset path

## Acceptance Criteria

- `brew install randlee/tap/sc-lint` succeeds and installs the full toolset
  binaries
- `release/publish-artifacts.toml` and `.github/workflows/release.yml` define a
  publish path for `sc-lint`, `sc-lint-boundary`, `sc-lint-portability`, and
  `sc-lint-runtime` through the tap-bootstrap + `sc-lint.rb` update path
- the sprint explicitly records the disposition of `homebrew-tap/Formula/sc-lint-boundary.rb`
  as either retired in this sprint or retained only as a legacy compatibility
  surface that is not the supported install path for normal users
- the CI `update-homebrew` job reaches its formula-generation and
  commit-or-noop path using the workflow checkout root `homebrew-tap/` without
  path-resolution errors
- Homebrew automation computes checksums from published release artifacts and
  updates `homebrew-tap/Formula/sc-lint.rb` deterministically for macOS Intel,
  macOS ARM, and Linux
- formula verification proves the selected Homebrew path exposes
  `sc-lint --version`, `sc-lint-boundary --version`,
  `sc-lint-portability --version`, and `sc-lint-runtime --version`
- user docs point at the primary `sc-lint` formula and explain that backend
  binaries are included in that install path
- the sprint doc makes the manifest/schema plan explicit enough that a
  developer knows whether `scripts/release_artifacts.py` needs code changes or
  only expanded data entries for the multi-binary release shape

## Required Validation

- `python3 scripts/release_artifacts.py validate-manifest --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
- `python3 scripts/release_artifacts.py validate-preflight-checks --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
- `python3 scripts/release_artifacts.py validate-publish-order --manifest release/publish-artifacts.toml --workspace-toml Cargo.toml`
- `cargo build --workspace`
- `ruby -c "${HOMEBREW_TAP_DIR}/Formula/sc-lint.rb"`
