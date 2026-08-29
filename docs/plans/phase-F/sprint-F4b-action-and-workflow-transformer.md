---
id: F.4b
title: Config-Derived GitHub Action And Workflow Transformer
status: planned
target: develop
---

# Sprint F.4b — Config-Derived GitHub Action And Workflow Transformer

## Goal

Close the reusable Action and optional workflow-generation boundary after F.4a
has made `sc-lint.toml` the sole configuration/version authority. F.4b owns
only Action acquisition/preflight and fixture-proven workflow generation; it
does not reopen the generic transaction, Just marker, or legacy-recipe work.

## Hard Dependencies

- F.1 Action/version-authority requirements and ADR-014;
- F.2 typed plan/conflict contract;
- F.4a accepted generated `sc-lint.toml`, `ManagedArtifact` transaction
  extension contract, and exported conflict/patch shapes;
- existing Phase E reusable Action release-artifact, checksum, and offline-doc
  contracts.

## Exact Targets

- `crates/sc-lint/src/configure/workflow.rs` (new)
- `action.yml`
- `action/index.js`
- `action/test/action.test.cjs`
- `docs/sc-lint/github-action-requirements.md`
- `tests/configure/test_workflow_transformer.py` (new)
- `tests/fixtures/configure/workflow/` (new)
- `tests/fixtures/configure/contracts/` (F.1-owned golden inputs consumed
  verbatim; no field redefinition)
- `docs-bundle/ci.md`

## Deliverables

- the reusable Action parses `[tool.sc-lint].minimum_version` from its
  `config-path`, selects and compatibility-preflights that exact released
  artifact, and exposes no independent selection input. A retained `version`
  input is optional assertion-only and fails before download when semantically
  unequal to config.
- F.4b synchronizes the F.1-owned Action version-authority amendment into the
  Action input table, fixtures, CI guide, and release behavior; it does not
  redefine the underlying product requirement or F.1 result/plan fields.
- the F.4b workflow transformer may create the standalone
  `.github/workflows/sc-lint.yml` only through an F.2/F.4a-approved plan. It
  uses `randlee/sc-lint@v1` with `setup`, `lint`, and `test`, points to the
  generated config, and contains no source checkout or copied script.
- an existing workflow is never parsed or changed by the shallow F.2 observer.
  F.4b recognizes only documented, fixture-proven workflow fingerprints. An
  unknown, near-match, or user-owned shape produces the F.1
  `manual_conflict`/exportable-patch contract with no write.
- generated workflow operations participate in the F.4a transaction and its
  digest recheck/rollback boundary. F.4b supplies the `WorkflowYamlArtifact`
  `ManagedArtifact` implementation, validates generated YAML before the
  transaction commits it, and does not implement a second transaction engine.

## Production-Ready Closure

Every listed deliverable must land production-ready for the F.4b Action and
workflow boundary, including all platforms, no-fallback behavior, and unknown
workflow conflicts. Nothing may be deferred except work explicitly listed in
[This Sprint Does Not Close](#this-sprint-does-not-close).

## Required Integration Sample

```yaml
- uses: randlee/sc-lint@v1
  with:
    operation: lint
    config-path: sc-lint.toml
```

The Action selects its archive from `[tool.sc-lint].minimum_version`; this is
not an optional implementation choice. A semantically unequal optional
`version` assertion returns the documented Action error envelope and cannot
select an alternate artifact.

## Acceptance Criteria

- Action tests prove config-derived selection, semantic assertion mismatch,
  archive/checksum verification, compatibility preflight, offline-doc output,
  Linux/macOS/Windows behavior, and absence of Cargo, source, package-manager,
  or analyzer fallback paths.
- a generated workflow is byte-deterministic from the approved plan and passes
  YAML validation. Reapply is idempotent; a changed digest, unknown workflow,
  or near-match produces the F.1 sample conflict/patch data and no write.
- every **configure workflow-plan** error uses the F.1 stable envelope and
  recovery reference; Action acquisition/runtime errors remain in the
  Action-requirements error family. Every generated workflow operation is
  committed or rolled back by F.4a's single transaction, including the F.4a
  synthetic-second-artifact extension scenario and a real `WorkflowYaml`
  rollback fixture.
- `docs-bundle/ci.md` documents only the released Action and the four-command
  consumer contract; it does not direct consumers to Cargo, a checkout, copied
  Python, or a manual installer.

## Required Validation

- Action fixture matrix for Linux, macOS, and Windows, including selection,
  mismatch, checksum, compatibility, and no-fallback cases
- workflow create/reapply/unknown/near-match/stale-plan/rollback fixtures
- YAML syntax validation and generated-Action documentation-link checks
- `just lint`
- `just test`

## This Sprint Does Not Close

- generic filesystem transactions, `sc-lint.toml` generation, Just markers,
  or legacy recipe migration, which F.4a owns;
- reference-consumer qualification or consumer PRs, which Phase P owns.
