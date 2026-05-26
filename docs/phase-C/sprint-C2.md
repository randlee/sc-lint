---
id: C.2
title: Shared Report Template Pipeline
status: planned
branch: feature/plan-sc-lint-version
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/feature/plan-sc-lint-version
target: develop
---

# Sprint C.2 — Shared Report Template Pipeline

## Goal

- define the shared report system consumed by interface-version artifacts and
  other future report producers
- ensure the report model follows the XHTML fragment/report pattern
- prevent drift toward hand-authored HTML documentation or feature-local HTML
  renderers

## Hard Dependencies

- [docs/phase-C/sprint-C1.md](./sprint-C1.md)
- [docs/sc-lint-version/requirements.md](../sc-lint-version/requirements.md)
- [docs/sc-lint/interface-reporting-constraints.md](../sc-lint/interface-reporting-constraints.md)

## Exact Targets

- `docs/phase-C/phase-C-plan.md`
- `docs/phase-C/sprint-C2.md`
- `docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md`
- `docs/sc-lint-version/requirements.md`
- `docs/sc-lint/interface-reporting-constraints.md`
- `docs/architecture.md`
- `docs/project-plan.md`

## Deliverables

- a planned report package model that includes:
  - main HTML report for human readers generated through a shared
    `sc-compose`/Jinja workflow rather than `sc-lint-version`-owned HTML code
  - JSON sidecar as the machine source of truth
  - separate XHTML section fragments/panels for section-level context
  - built-in copy actions per XHTML panel for canonical JSON and canonical
    context text
- one planned ownership split:
  - `sc-lint-version` owns canonical interface artifacts, baseline diffs, and
    version verdicts
  - the reusable report renderer/templates live outside `sc-lint-version`
    itself, with the `sc-compose` repo as the preferred ownership target
    because the same report conventions are expected to serve non-lint tools
- one planned reusable template family set for:
  - Rust public API reports
  - stable top-level CLI contract reports
  - ICD-style RPC/socket reports when present
- one planned CLI baseline artifact definition that includes:
  - a versioned JSON schema for command ids, required request/response
    fields, and stable machine error codes
  - generation through
    `sc-lint check interfaces --family cli --write-baseline <path>`
  - one explicit replacement workflow for approved major-version changes
- one planned template-selection and override surface under
  `[reporting.templates.<report_kind>]` so consumers can point to alternate
  Jinja templates without editing generated HTML code
- explicit planning language that generated templates and structured data own
  the output, not manually maintained HTML pages

## Explicit Code Samples

```json
{
  "schema": "sc-lint-cli-interface-v1",
  "report_kind": "cli",
  "output_path": "artifacts/interfaces/sc-lint-cli/index.html",
  "json_output_path": "artifacts/interfaces/sc-lint-cli/index.json",
  "title": "sc-lint CLI Interface Report",
  "sections": [
    {
      "id": "lint-sc-boundary",
      "title": "lint.sc-boundary",
      "xhtml_path": "artifacts/interfaces/cli/sections/lint-sc-boundary.xhtml"
    }
  ]
}
```

```toml
[reporting.templates."public-api"]
source = "sc-compose:sc-lint-public-api-report"

[reporting.templates.cli]
path = ".sc-lint/templates/cli-report.html.j2"

[reporting.templates.icd]
path = ".sc-lint/templates/icd-report.html.j2"
```

```text
artifacts/interfaces/
  crate-api/
  cli/
  transport/

artifacts/baselines/
  cli-v0.2.0.json
```

## This Sprint Does Not Close

- the semver decision logic for Rust APIs
- hard-fail policy enforcement in CI
- final implementation of the shared reporting layer in `sc-compose`
- consumer-onboarding skill or marketplace delivery

## Acceptance Criteria

- `docs/sc-lint/interface-reporting-constraints.md` states that the report
  package is generated from structured data, `sc-compose render`, and reusable
  Jinja templates rather than hand-written HTML
- `docs/sc-lint-version/requirements.md` requires a JSON sidecar as the
  canonical machine-readable output
- `docs/sc-lint/interface-reporting-constraints.md` requires separate XHTML
  section fragments/panels and built-in copy actions per panel
- `docs/sc-lint-version/requirements.md`,
  `docs/phase-C/phase-C-plan.md`, and
  `docs/sc-lint/adr/ADR-011-interface-versioning-and-published-artifacts.md`
  all state that `sc-lint-version` does not own a feature-local HTML renderer
  and instead consumes a shared reporting layer whose preferred ownership
  target is the `sc-compose` repo
- `docs/sc-lint-version/requirements.md` defines a template-selection and
  override contract under `[reporting.templates.<report_kind>]`
- `docs/sc-lint-version/requirements.md` defines template families for
  `public-api`, `cli`, and `icd`
- `docs/sc-lint-version/requirements.md` requires published coverage across
  all shipped crates, not only the top-level crate
- `docs/sc-lint-version/requirements.md` defines the CLI baseline artifact
  schema, the
  `sc-lint check interfaces --family cli --write-baseline <path>`
  generation workflow, and the approved baseline-replacement path for major
  version updates
- `docs/sc-lint-version/requirements.md` is updated to keep the CLI baseline
  acceptance path explicitly tied to `REQ-VERSION-012A`,
  `REQ-VERSION-012B`, and `REQ-VERSION-012C`

## Required Validation

- `just lint`
