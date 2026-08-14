---
id: F.4
title: Safe Consumer Integration And CI Replacement Transformers
status: planned
target: develop
---

# Sprint F.4 — Safe Consumer Integration And CI Replacement Transformers

## Goal

Turn an approved F.2/F.3 plan into a verified transaction that installs the
product-owned integration into both empty and established repositories. F.4 is
where safety is enforced, not delegated to the wizard.

## Hard Dependencies

- F.1 contracts/ADR-014
- F.2 typed plan/digest/conflict engine
- F.3 normalized requests and final-confirmation front end
- existing Phase E bootstrap, installer, config, documentation, and Action

## Exact Targets

- `crates/sc-lint/src/configure/apply.rs` (new)
- `crates/sc-lint/src/configure/just.rs` (new)
- `crates/sc-lint/src/configure/workflow.rs` (new)
- `crates/sc-lint/src/configure/legacy.rs` (new)
- `crates/sc-lint/src/consumer_integration.rs`
- `crates/sc-lint/assets/consumer-Justfile`
- `crates/sc-lint/assets/consumer-config.toml`
- `action.yml`
- `action/index.js`
- `action/test/action.test.cjs`
- configuration/apply fixtures and tests
- `docs-bundle/just-setup.md`
- `docs-bundle/ci.md`
- `docs-bundle/upgrade.md`
- `docs-bundle/troubleshooting.md`
- `docs/sc-lint/github-action-requirements.md`

## Deliverables

- `configure --apply` accepts only a plan whose identifier and source digests
  match a freshly recomputed plan. It stages all outputs beside their targets,
  applies an ordered transaction, validates every changed TOML/Just/YAML/JSON
  artifact, and restores prior bytes/modes on any failure. A partial rollback
  is its own stable, actionable error with backup paths.
- empty repositories receive the canonical Phase E `sc-lint.toml`, bootstrap
  helpers, and root Justfile path. Established repositories with no existing
  `setup`, `lint`, `test`, or `upgrade` recipe receive the same config/bootstrap
  plus `.sc-lint/justfile` and exactly one generated marker block/import in
  their root Justfile. Reapplying is idempotent.
- a managed marker block has a documented begin/end signature, canonical
  import form, and exact ownership rule. If missing, duplicated, moved, or
  modified outside an approved generated representation, it is a conflict;
  no whole-file Justfile replacement is permitted.
- generated recipes provide `setup`, `lint`, `test`, and `upgrade` and are
  compatible with Just's documented import behavior. If an established root
  already defines any reserved recipe, import precedence/duplicate-recipe
  behavior must never be used to silently shadow it. The tool either applies a
  fingerprinted migration that replaces that exact legacy recipe with the
  canonical managed behavior, or reports a no-write conflict. It must not
  create aliases that leave `just lint` or `just test` noncanonical.
- generated `sc-lint.toml` has the sole minimum-version authority and complete
  selected argv profiles. The Action obtains the exact release version by
  parsing that config; `version` becomes optional assertion-only input. A
  mismatch fails before download and cannot select a different artifact.
- a generated standalone `.github/workflows/sc-lint.yml` uses
  `randlee/sc-lint@v1` with setup/lint/test operations and no copied source
  scripts. Existing workflow changes are supported only for F.4-recognized,
  fixture-proven shapes and are presented as a bounded patch; unknown YAML
  remains untouched.
- legacy deletion is an allowlisted, digest-checked operation. The initial
  allowlist covers the exact sc-compose 0.4 custom `setup-sc-lint`,
  `setup-lint-toolchain`, copied `.just` artifacts, and manual consumer
  workaround only after their replacements validate. The transformer must not
  delete arbitrary files named similarly.

## Required Integration Samples

```just
# >>> sc-lint managed integration >>>
import '.sc-lint/justfile'
# <<< sc-lint managed integration <<<
```

```yaml
- uses: randlee/sc-lint@v1
  with:
    operation: lint
    config-path: sc-lint.toml
```

The Action derives the selected release from `[tool.sc-lint].minimum_version`.
An optional `version` assertion is valid only when semantically equal to that
field and is retained solely for migration diagnostics, not selection.

## Acceptance Criteria

- a successful apply leaves only the reviewed plan changes; a failed write,
  parser validation, or post-write check restores all prior bytes and modes.
- a file modified after plan creation is never written; the stale-plan response
  names the path and tells the caller to regenerate/review the plan.
- an established Justfile preserves every byte outside the generated marker
  range and any exact fingerprinted recipe replacement; fixtures cover
  comments, recipes, imports, CRLF, Windows paths, and every reserved-recipe
  collision class.
- neither `configure` nor generated recipes write a README, invoke `cargo run`,
  copy `.just/*.py`, or download a source archive.
- Action tests prove config-derived selection, assertion mismatch, archive
  verification, Linux/macOS/Windows behavior, and absence of fallback paths.
- F.4, not the shallow F.2 context collector, recognizes every supported legacy
  migration fingerprint. A migration is impossible to apply to a near-match
  fixture.

## Required Validation

- transaction fault-injection tests at each write/rename/validation stage
- TOML, Just, YAML, and JSON syntax validation fixtures
- Action fixture matrix for Linux/macOS/Windows
- empty/existing/marker-conflict/stale-plan/legacy-near-miss fixtures
- `just lint`
- `just test`

## This Sprint Does Not Close

- final conversion of the live `sc-compose` repository; F.5 owns that
  consumer acceptance proof and any newly exposed product defect.
