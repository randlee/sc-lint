---
id: F.5
title: sc-compose Reference Conversion And Consumer Setup Closure
status: planned
target: develop
---

# Sprint F.5 — sc-compose Reference Conversion And Consumer Setup Closure

## Goal

Prove that Phase F is a real replacement, not a new theoretical interface, by
using the released `sc-lint configure` product to convert `sc-compose`. The
conversion is successful only when sc-compose becomes the copyable reference
consumer and requires no repository-specific setup workaround.

## Hard Dependencies

- F.1 through F.4
- a released sc-lint version containing the F.4 configuration/Action contract
- a dedicated, coordinated sc-compose worktree and explicit sc-compose team
  acceptance (no cross-repository direct write from the sc-lint worktree)

## Exact Targets

### sc-lint repository

- Phase F plans and requirements/ADR/architecture docs
- consumer lifecycle/acceptance fixtures
- docs-bundle setup, Just, CI, upgrade, troubleshooting, and best-practice
  guides
- root `README.md`, `AGENTS.md`, and Phase F project-plan/roadmap entries
- reusable Action docs/tests/release validation

### sc-compose acceptance worktree

- `sc-lint.toml`
- `Justfile` and generated `.sc-lint/` assets
- `.github/workflows/ci.yml` and obsolete local setup Action assets as planned
- copied `.just` artifacts and only exact recognized legacy assets
- sc-compose CI/workflow tests and consumer acceptance evidence

## Deliverables

- a checked-in JSON request is created from sc-compose's reviewed selected
  pages; it is an auditable input to `sc-lint configure --request <file>
  --dry-run --json` and an example
  for agent-driven adoption. It uses one configured minimum version and does
  not contain executable shell strings.
- a separate sc-compose PR applies the exact generated plan, with the
  tool-produced plan ID, diffs, preconditions, and removals captured in the PR
  evidence. No operator manually edits the generated integration to complete
  the conversion.
- the old top-level `version = "0.4.0"`, Action default/pin duplication,
  private release downloader, source archive download, and copied
  `.just/*.py` utilities are removed. The hand-built `lint-ci-consumer`
  workaround is removed or replaced solely by the configured complete product
  profile; no behavior is silently discarded.
- sc-compose retains only explicitly product-specific reporting behavior, if
  any. It must sit after/beside the standard `just lint`/`just test` contract,
  not install or orchestrate sc-lint.
- local use, Action CI, and a clean checkout demonstrate the same one-command
  developer contract. The config version is the only selection authority in
  every path.
- sc-lint documentation adds a before/after conversion case study and an
  agent JSON example based on the sanitized sc-compose request. It explains
  how established Justfiles are preserved, how to use preview/apply, how to
  inspect conflicts, and when a consumer should stop and report a product gap.

## Acceptance Criteria

- sc-compose conversion completes through `configure` preview then apply with
  no manual wiring of Just, installer, copied Python, or release URL.
- `just setup`, `just lint`, `just test`, and `just upgrade` run from a fresh
  sc-compose checkout with only documented prerequisites.
- sc-compose CI uses the reusable Action/config-derived release selection;
  it never fetches an sc-lint source archive or runs consumer copied
  implementation scripts.
- grep/fixture gate proves the removed legacy patterns are absent from active
  sc-compose setup/CI paths; a separate evidence list records any intentionally
  retained sc-compose-specific report feature and why it is not sc-lint setup.
- the conversion and lifecycle suite pass on Linux, macOS, and Windows. A
  failure on one OS blocks closure.
- if sc-compose exposes an unsupported existing-repository shape, F.5 opens a
  concrete sc-lint finding and returns work to F.2/F.4; it does not authorize a
  manual consumer workaround or a false close.

## Required Validation

- sc-lint: `just lint`, `just test`, configure fixture matrix, Action matrix,
  documentation/link validation
- sc-compose: generated plan `--dry-run`, apply/reapply check, `just setup`,
  `just lint`, `just test`, and its complete CI matrix
- released binary / Homebrew installation smoke and reusable Action execution
  on Linux, macOS, and Windows
- post-conversion source scan proving no active copied source utilities/custom
  installer/manual 0.4 profile remains

## This Sprint Does Not Close

- arbitrary consumer migrations beyond the supported contract;
- changes to sc-compose reporting semantics that are not necessary to separate
  reporting from setup/lint/test ownership;
- Action secret management, auto-commit, or auto-PR behavior.
