---
id: F.4a
title: Transactional Configure Apply And Just Integration
status: planned
target: develop
---

# Sprint F.4a — Transactional Configure Apply And Just Integration

## Goal

Turn an approved F.2/F.3 plan into a verified transaction that installs the
product-owned config and Just integration into both empty and established
repositories. F.4a owns safe mutation and Just coexistence; the Action and
workflow boundary is separately closed by F.4b.

## Hard Dependencies

- F.1 contracts/ADR-014
- F.2 typed plan/digest/conflict engine
- F.3e normalized requests, qualified wizard final-confirmation front end, and
  cross-adapter equivalence evidence
- existing Phase E bootstrap, installer, config, and documentation

## Exact Targets

- `crates/sc-lint/src/configure/apply.rs` (new)
- `crates/sc-lint/src/configure/just.rs` (new)
- `crates/sc-lint/src/configure/legacy.rs` (new)
- `crates/sc-lint/src/consumer_integration.rs`
- `crates/sc-lint/assets/consumer-Justfile`
- `crates/sc-lint/assets/consumer-config.toml`
- `tests/configure/test_apply_and_just.py` (new)
- `tests/fixtures/configure/apply-and-just/` (new)
- `docs-bundle/just-setup.md`
- `docs-bundle/upgrade.md`
- `docs-bundle/troubleshooting.md`

## Deliverables

- `configure --apply` accepts only a plan whose identifier and source digests
  match a freshly recomputed plan. It stages all outputs beside their targets,
  applies an ordered transaction, validates every changed TOML, Just, and JSON
  artifact, and restores prior bytes/modes on any failure. F.4b validates a
  generated YAML workflow before passing it to this transaction. A partial
  rollback is its own stable, actionable error with backup paths. Staging,
  digest recheck, commit, and rollback apply uniformly to every generated
  artifact type, including that pre-validated YAML workflow.
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
  selected argv profiles. F.4b consumes that file as the Action's immutable
  selection authority; F.4a neither modifies a workflow nor accepts a second
  version value.
- legacy deletion is an allowlisted, digest-checked operation. The initial
  allowlist covers the exact sc-compose 0.4 custom `setup-sc-lint`,
  `setup-lint-toolchain`, copied `.just` artifacts, and manual consumer
  workaround only after their replacements validate. The transformer must not
  delete arbitrary files named similarly.

## Production-Ready Closure

Every listed deliverable must land production-ready for the F.4a transaction
and Just boundary, including failure paths and exact legacy near-misses.
Nothing may be deferred except work explicitly listed in
[This Sprint Does Not Close](#this-sprint-does-not-close).

## Required Integration Sample

```just
# >>> sc-lint managed integration >>>
import '.sc-lint/justfile'
# <<< sc-lint managed integration <<<
```

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
- F.4a, not the shallow F.2 context collector, recognizes every supported legacy
  migration fingerprint. A migration is impossible to apply to a near-match
  fixture.
- every plan operation, conflict, exportable patch, and stable apply error
  exactly matches the F.1 contract samples; F.4a does not invent a local error
  or patch shape.

## Required Validation

- transaction fault-injection tests at each write/rename/validation stage
- TOML, Just, and JSON syntax validation fixtures
- empty/existing/marker-conflict/stale-plan/legacy-near-miss fixtures
- `just lint`
- `just test`

## This Sprint Does Not Close

- final conversion of either reference consumer; Phase P owns that
  dual-consumer acceptance proof and returns any newly exposed product defect
  to F.2/F.4a;
- GitHub Action behavior or `.github/workflows/sc-lint.yml` generation, which
  F.4b owns.
