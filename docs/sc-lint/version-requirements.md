# `sc-lint-version` Requirements

This document defines the planned requirements for `sc-lint-version`, the
interface-versioning and published-interface artifact capability for
`sc-lint`.

## Purpose

`sc-lint-version` exists to do two things from one canonical interface
inventory:

1. produce human-friendly published interface documentation
2. hard-fail when a breaking interface change is introduced without an
   approved versioning action

The initial release line is based on `cargo-semver-checks` for Rust crate
public API analysis and extends the same versioning discipline to the stable
top-level CLI contract and any RPC/socket surfaces that exist now or are added
later.

## Interface Families

- `REQ-VERSION-001`
  `sc-lint-version` must treat interface versioning as a multi-surface
  capability rather than a Rust-library-only check.

- `REQ-VERSION-002`
  The initial supported interface families are:
  - Rust public APIs for all shipped crates
  - stable top-level CLI commands and machine contracts
  - RPC/socket interfaces, when the repo defines any

- `REQ-VERSION-003`
  Each supported interface family must have both:
  - a machine-readable canonical artifact used for change detection
  - a human-friendly published report derived from the same structured source

## Invocation And Configuration

- `REQ-VERSION-003A`
  The planned top-level invocation path for interface-version checks is
  `sc-lint check interfaces`; the existing `sc-lint version` command remains
  reserved for reporting the tool version itself.

- `REQ-VERSION-003B`
  The planned Phase `C` form-factor for this capability is a dedicated
  workspace crate named `sc-lint-version`, integrated into the top-level CLI
  through `sc-lint check interfaces`.

- `REQ-VERSION-003C`
  Interface-family selection must use one canonical configuration surface
  inside `sc-lint` config under `[version.families.<family>]`.

- `REQ-VERSION-003D`
  Only family tables explicitly declared under `[version.families]` are part
  of a given run; omitted families are outside the run rather than implicitly
  reported as absent.

- `REQ-VERSION-003E`
  When a family is configured under `[version.families.<family>]` but the repo
  does not currently define that surface, the verdict must report
  `not_present` or equivalent rather than silently dropping the family.

- `REQ-VERSION-003F`
  Canonical interface-family identifiers must keep the same spelling across
  config, CLI flags, baseline artifacts, and verdict output so one family does
  not silently acquire multiple names.

## Artifact Model

- `REQ-VERSION-004`
  Human-facing published interface documentation must be generated from
  structured data and reusable templates rather than hand-written monolithic
  HTML files.

- `REQ-VERSION-005`
  Generated human-facing report packages must follow the XHTML fragment/report
  pattern summarized in
  [docs/sc-lint/interface-reporting-constraints.md](./interface-reporting-constraints.md):
  - one self-contained main HTML report
  - one JSON sidecar as the machine-readable source of truth
  - separate XHTML section fragments/panels for deeper per-section context

- `REQ-VERSION-006`
  The JSON sidecar must remain the authoritative machine-readable baseline for
  change detection; HTML and XHTML outputs are presentation artifacts derived
  from it.

- `REQ-VERSION-007`
  Every report package must be reproducible from structured inputs and
  templates alone and must not depend on manual HTML patching after render.

- `REQ-VERSION-007A`
  The generated report pipeline must use the repo-local reusable
  HTML-report workflow described in
  [docs/sc-lint/interface-reporting-constraints.md](./interface-reporting-constraints.md)
  rather than a separate ad hoc HTML rendering path.

- `REQ-VERSION-007B`
  Each XHTML section fragment/panel must expose built-in copy actions for the
  canonical section JSON payload and canonical section context text.

## Rust Public API Rules

- `REQ-VERSION-008`
  The initial Rust public API comparison engine must be `cargo-semver-checks`.

- `REQ-VERSION-008A`
  The `rust-public-api` family must consume `cargo-semver-checks` through one
  explicit `sc-lint-version` translation layer that reads the tool's
  machine-readable output mode and exit-status semantics.

- `REQ-VERSION-008B`
  That translation layer must map the `cargo-semver-checks` result into one
  per-family verdict record, including the `breaking_items` list used by
  `REQ-VERSION-017A`.

- `REQ-VERSION-009`
  For Rust public APIs, a breaking change is detected when
  `cargo-semver-checks` reports a deny-level semver violation against the
  configured baseline.

- `REQ-VERSION-010`
  The baseline for Rust public API checks must be configurable so the product
  can compare against:
  - the latest published crates.io release
  - a specific published version
  - a specific git revision
  - an explicit baseline artifact set

- `REQ-VERSION-011`
  `sc-lint-version` must not redefine Rust semver semantics independently when
  `cargo-semver-checks` already provides a supported decision.

## CLI Interface Rules

- `REQ-VERSION-012`
  The stable top-level CLI command surface must be versioned as an explicit
  interface family separate from Rust crate public APIs.

- `REQ-VERSION-012A`
  The CLI family baseline must use a versioned JSON artifact schema that
  records stable command identifiers, required request fields, required
  response fields, and stable machine-readable error codes.

- `REQ-VERSION-012B`
  The planned CLI baseline-generation workflow is
  `sc-lint check interfaces --family cli --write-baseline <path>`, which
  emits the versioned JSON baseline artifact used for later comparisons.

- `REQ-VERSION-012C`
  Approved major-version CLI changes must replace the stored CLI baseline by
  rerunning the baseline-generation workflow and reviewing the updated
  versioned artifact.

- `REQ-VERSION-013`
  For the CLI family, a breaking change is detected when any of the following
  occur in the canonical artifact set without an approved major-version path:
  - removal or rename of a stable command identifier
  - removal or incompatible type change of a required request field
  - removal or incompatible type change of a required response field
  - removal or incompatible repurposing of a stable machine error code
  - removal of a stable non-interactive command family from the published
    contract surface

- `REQ-VERSION-014`
  Additive CLI changes may be classified separately from breaking changes, but
  they must still appear in the published report and machine-readable diff.

## RPC/Socket Interface Rules

- `REQ-VERSION-015`
  RPC/socket interfaces must use the same artifact discipline when such
  surfaces exist in the repo.

- `REQ-VERSION-016`
  For RPC/socket interfaces, a breaking change is detected when any of the
  following occur in the canonical artifact set without an approved major
  version path:
  - removal or rename of an operation, message kind, or handshake step
  - incompatible change to required fields or field types
  - incompatible change to framing, transport contract, or wire-level
    negotiation semantics
  - removal of a documented required server or client capability

## Hard-Fail Policy

- `REQ-VERSION-017`
  `sc-lint-version` must provide a hard-fail mode that exits non-zero when a
  detected breaking change is present in any enabled interface family.

- `REQ-VERSION-017A`
  The multi-family hard-fail result must use one top-level verdict record for
  the full run that includes:
  - `ok` as the aggregate rollup for the run
  - one per-family verdict entry for every enabled interface family
  - explicit `not_present` or equivalent status for configured families that
    are absent in the current repo
  The aggregate `ok` value must be `false` when any enabled family reports a
  breaking change.

- `REQ-VERSION-018`
  The hard-fail verdict must identify:
  - the interface family
  - the baseline used
  - the specific breaking items
  - the published artifact paths associated with the failure

- `REQ-VERSION-019`
  The hard-fail mechanism must be usable in local developer workflows and CI.

## Publication Requirements

- `REQ-VERSION-020`
  Published interface reports must be understandable by developers and users
  without requiring them to inspect raw machine diffs first.

- `REQ-VERSION-021`
  The published report set must cover all shipped crates, not only the
  top-level `sc-lint` crate.

- `REQ-VERSION-022`
  When an interface family is not present in the repo, the published artifact
  set must state that explicitly rather than silently omitting the family.

- `REQ-VERSION-022A`
  The planning and published-documentation line must include one clear consumer
  integration document describing what a consuming repository must provide to
  exercise `sc-lint-version` for:
  - CLI surfaces
  - Rust public API baselines
  - RPC/socket interfaces when present

- `REQ-VERSION-022AA`
  That consumer integration document must explicitly describe the expected
  repo-side harness, fixture, simulator, and normalization responsibilities,
  including the rule that consuming repos should reuse existing CLI
  testability and simulator/transcript infrastructure where available instead
  of rebuilding bespoke interface exercisers.

- `REQ-VERSION-022B`
  The consumer integration guidance must be packaged as a repo-local Claude
  Code skill so the adoption workflow is discoverable and reusable.

- `REQ-VERSION-022C`
  The skill-design sprint and the minimal-marketplace sprint must remain
  separate planning closures.

- `REQ-VERSION-022D`
  The repo-local Claude Code skill must be advertised through a minimal
  repo-local Claude Code marketplace rather than relying on ad hoc path
  knowledge.

- `REQ-VERSION-022E`
  The minimal-marketplace planning line must name both source-repo publication
  surfaces required by the marketplace design described in
  [docs/sc-lint/minimal-marketplace-constraints.md](./minimal-marketplace-constraints.md):
  - `.claude-plugin/marketplace.json`
  - `packages/sc-lint-version-adoption/.claude-plugin/plugin.json`

## Non-Goals

- `REQ-VERSION-023`
  The initial planning line does not require a brand-new semver engine for
  Rust crate APIs.

- `REQ-VERSION-024`
  The initial planning line does not allow hand-maintained HTML snapshots to
  become the canonical version baseline.
