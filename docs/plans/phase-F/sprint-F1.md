---
id: F.1
title: Setup Automation Contract And Architecture Foundation
status: planned
target: develop
---

# Sprint F.1 — Setup Automation Contract And Architecture Foundation

## Goal

Define the production contract for the sc-lint-owned consumer configuration
tool before code is written. This sprint closes ambiguity about its small MVP
surface, JSON schemas, user-visible explanation, ownership, later file
mutation, Justfile coexistence, and GitHub-Action version selection.

## Hard Dependencies

- `docs/phase-E/phase-E-plan.md`
- `docs/requirements.md` `REQ-PRODUCT-019` through `022`
- `docs/sc-lint/cli-requirements.md` `REQ-CLI-019` through `024`
- `docs/sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md`
- `docs/sc-lint/github-action-requirements.md`
- Phase F plan in this directory

## Exact Targets

- `docs/plans/phase-F/phase-F-plan.md`
- `docs/plans/phase-F/sprint-F1.md`
- `docs/requirements.md`
- `docs/sc-lint/cli-requirements.md`
- `docs/sc-lint/cli-contract.md`
- `docs/sc-lint/cli-architecture.md`
- `docs/architecture.md`
- `docs/sc-lint/github-action-requirements.md`
- `docs/sc-lint/adr/ADR-014-consumer-configuration-automation.md` (new)
- `docs/sc-lint/adr/README.md`
- `docs/project-plan.md`
- `docs/sc-lint/roadmap.md`

## Deliverables

- `REQ-PRODUCT-023` defines product-owned consumer configuration, discovery,
  plan/apply transaction, preservation, noninteractive JSON, and required
  cross-platform acceptance.
- `REQ-CLI-025` through `REQ-CLI-028` define `configure`, its versioned JSON
  request, stable plan/result envelope, UI selection, and error family.
- ADR-014 accepts the thin Python/Wyvern MVP architecture: conventional file
  presence becomes versioned JSON context, the UI displays it and collects
  choices, and product-owned apply comes only after a reviewed plan. It
  explicitly amends ADR-012: `init --just` retains its four-file exact
  ownership; a later `configure` apply may own `.sc-lint/justfile`, an optional
  generated workflow, and only a bounded marker block in an existing Justfile.
- the Action requirements replace its independent required `version` input
  with config-derived version selection; an optional asserted version may only
  verify equality with config and cannot override it.
- the CLI contract names the `configure`, `configure.plan`, and
  `configure.apply` command identities and error codes, including unsupported
  request schema, UI unavailable, unmanaged collision, stale plan, and
  transaction rollback failure.
- every public context/request/plan type has a JSON Schema, a JSON example, and
  an explanation of its user-facing purpose. The MVP has no general repository
  parser: its context schema contains only documented conventional path facts.

## Required Contract Samples

```json
{
  "schema_version": "v1",
  "context": {
    "cargo_toml": {"present": true, "kind": "workspace"},
    "sc_lint_toml": {"present": false},
    "justfile": {"present": true, "inspected": false},
    "github_workflows": {"present": true, "inspected": false},
    "sc_lint_directory": {"present": false}
  },
  "explanation": {
    "developer_contract": ["just setup", "just lint", "just test", "just upgrade"],
    "uninspected_existing_integration": ["Justfile", ".github/workflows/"]
  },
  "request": {
    "minimum_version": "0.5.0",
    "lint_families": {
      "baseline": {"state": "recommended"},
      "boundary": {"state": "enabled", "settings": {"inventory": "detect"}},
      "portability": {"state": "enabled"},
      "runtime": {"state": "disabled"},
      "attributes": {"state": "recommended"}
    },
    "just": {"mode": "keep_existing"},
    "ci": {"mode": "keep_existing"}
  }
}
```

The initial recommended profile expansion is also fixed so a later UI cannot
invent an implementation path:

```toml
[[tool.sc-lint.lint]]
name = "fmt"
command = ["cargo", "fmt", "--all", "--check"]

[[tool.sc-lint.lint]]
name = "sc-boundary"
command = ["sc-lint", "lint", "sc-boundary"]

[[tool.sc-lint.test]]
name = "workspace"
command = ["cargo", "test", "--workspace"]
```

The portability/runtime pages use the same installed-product command form when
enabled. The attributes/directives page has no `[[tool.sc-lint.lint]]` command
until a separate shipped executable/profile contract exists.

```json
{
  "ok": true,
  "command": "configure.plan",
  "data": {
    "plan_id": "sha256:...",
    "operations": [{"path": ".sc-lint/justfile", "kind": "propose_create"}],
    "conflicts": [],
    "manual_steps": []
  },
  "diagnostics": []
}
```

## Acceptance Criteria

- a QA reader can determine the MVP's bounded checks, explanation pages, JSON,
  file ownership, and Action version behavior from this sprint document and
  its named contract files.
- the plan distinguishes an empty-repository initializer from an existing-repo
  conversion; neither silently overwrites an arbitrary Justfile or README.
- every lint-family page is named and its recommendation/accept/modify/disable
  semantics are represented in the JSON schema.
- the schema has an explicit `inspected: false` representation for existing
  Justfile/workflow content so users cannot mistake a presence check for a
  compatibility analysis.
- all mutations have preconditions, digest recheck, validation, rollback, and
  stable recovery semantics.
- the Action has exactly one version policy authority: `sc-lint.toml`.
- ADR-014 and requirements/architecture/roadmap/project-plan updates land in
  the same implementation PR as the contract types.

## Required Validation

- markdown link validation
- contract-schema fixture review against the samples above
- `just lint`
- `just test`

## This Sprint Does Not Close

- implementation of discovery, the wizard, file mutation, Action changes, or
  sc-compose conversion; those belong to F.2 through F.5.
