---
id: E.4
title: Documentation Bundle, Package Guides, And Help Discovery
status: implemented
branch: feature/phase-E4-distributed-documentation
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/phase-E4-distributed-documentation
target: integrate/phase-E
---

# Sprint E.4 — Documentation Bundle, Package Guides, And Help Discovery

## Goal

Create the versioned static documentation bundle that is installed with the
product and discoverable through `sc-lint` help, including a canonical guide
for the recommended `just` setup.

## Governing Plan

- [Phase E plan](./phase-E-plan.md)

## Hard Dependencies

- [Sprint E.1](./sprint-E1.md) version/error terminology
- [Sprint E.2](./sprint-E2.md) installation/upgrade commands and failure codes
- [Sprint E.3](./sprint-E3.md) canonical Just/init contract
- current root `README.md` and seven package README files
- `crates/sc-lint/src/lib.rs` help rendering
- existing release artifact and Homebrew planning in `docs/phase-B/`

## Exact Targets

- a versioned documentation-bundle source directory
- root documentation source, operator guides, and package guide sources
- `crates/sc-lint/src/cli.rs`, `command.rs`, `lib.rs`, and tests as required
  for `docs`/help discovery
- `docs/requirements.md` (`REQ-PRODUCT-021`)
- `docs/sc-lint/cli-requirements.md`
- `README.md` and `docs/sc-lint/README.md` indexes
- `docs/phase-E/sprint-E4.md`

## Bundle Layout

```text
docs-bundle/
  README.md
  installation.md
  using-sc-lint.md
  configuration.md
  just-setup.md
  ci.md
  upgrade.md
  troubleshooting.md
  best-practices.md
  packages/
    sc-lint.md
    sc-lint-attributes.md
    sc-lint-boundary.md
    sc-lint-directives.md
    sc-lint-portability.md
    sc-lint-runtime.md
    sc-lint-schema.md
```

The final source-directory name may change, but the installed bundle must
preserve this logical layout. `README.md` is the bundle overview; it is never
written to a consumer repository root.

## Deliverables

- one guide exists for every published `sc-lint-*` package, including
  library-only packages; guide names and package identities are validated
  against the release manifest.
- the bundle overview indexes the complete operator manual. The manual has
  dedicated guides for installation, first use and daily use, configuration,
  Just setup, CI, upgrading, troubleshooting, and best practices; each guide
  has copyable commands and links to the relevant package guides.
- `installation.md` covers Homebrew, release-installer, and CI/Action paths;
  `using-sc-lint.md` explains `just setup`, `just lint`, and `just test`;
  `configuration.md` explains the minimum version and policy settings;
  `troubleshooting.md` maps every E.1/E.2/E.3 stable failure code to cause and
  recovery; `best-practices.md` defines the recommended agent/CI workflow.
- `just-setup.md` is the canonical copy/paste and generated-template guide. It
  defines `setup`, `lint`, `test`, and `upgrade`, the private preflight, the
  source-maintainer versus consumer distinction, and the one-command
  `sc-lint init --just` path for a new consumer repository.
- every package guide covers purpose, intended users, configuration/inputs,
  commands or API surface, finding/output interpretation, CI use, common
  failures, and links to related packages. Library-only packages receive the
  same level of ownership/usage guidance rather than a placeholder page.
- `sc-lint help` includes a documentation section, and a `sc-lint docs` family
  lists the documentation root, resolves a named package guide, and can print
  the path for automation. It must work without network access.
- unavailable/missing documentation is a structured `RBP-001` failure with
  bundle path, recovery action, and docs/install reference.
- docs use relative, packageable links and pass a link/manifest validation
  gate before release packaging.

## Acceptance Criteria

- help output identifies the overview, `just` setup guide, and every package
  guide without requiring users to inspect the source checkout.
- package-guide completeness test fails if a publishable package is added or
  renamed without a corresponding guide.
- `sc-lint docs --path` resolves an installed path and does not mutate the
  current repository.
- the canonical Just document and `sc-lint init` template stay byte-for-byte
  synchronized or are generated from one source.
- operator-manual coverage test fails if any mandatory guide is missing, or if
  a stable installation/compatibility error lacks a troubleshooting entry.
- `REQ-PRODUCT-021` and CLI requirements explicitly require installed
  documentation, the operator manual, package-guide completeness, and
  offline help discovery.

## Required Validation

- documentation manifest/package completeness test
- Markdown link check against the staged bundle
- `sc-lint --help`, `sc-lint docs`, `sc-lint docs just-setup`, and each package
  lookup tested from an installation-style fixture
- `cargo test -p sc-lint`

## Implementation Record

- `docs-bundle/` is the versioned operator manual source with a manifest,
  relative-link gate, and one guide for every publishable workspace package.
- `sc-lint docs [guide] [--path]` discovers the physical bundle beside the
  executable, in Homebrew-style `share/sc-lint`, or in the source checkout;
  it performs no network access or repository mutation and reports
  `CLI.SC_LINT_DOCS_UNAVAILABLE` when the bundle is absent.
- Top-level help names the overview, Just guide, operator guides, and package
  guides. `sc-lint docs` prints the overview; named guides and path mode are
  covered by the CLI and bundle tests.
