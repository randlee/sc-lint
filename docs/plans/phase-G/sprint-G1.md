---
id: G.1
title: sc-lint Adoption Kit
status: planned
branch: sprint/G.1-adoption-kit
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.1-adoption-kit
stack: A
stack_base: sprint/G.0-abandon-phase-F
target: develop (via stack A, PR base sprint/G.0-abandon-phase-F)
owner: clint
---

# Sprint G.1 — sc-lint Adoption Kit

## Goal

- ship `packages/sc-lint-adoption`, a vendorable kit in the `sc-publish` form,
  that installs the ADR-012 consumer end state into any Rust repository and
  reports drift

## Hard Dependencies

- G.0's **Unblock Milestone** committed on
  `sprint/G.0-abandon-phase-F`; this sprint's PR base is that branch, so
  implementation may begin before G.0 CI, QA, review, or merge.
- ADR-015 (accepted on the Phase G planning branch before orchestration)
- [ADR-012](../../sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md)
- reference contract: `../sc-publish/plugins/sc-publish/install.py`
  (`--input`, `--dry-run`, positional repo, byte-for-byte copy set,
  `RENAMED_FILES`, `TEMPLATES`, unified-diff drift, exit 1 on drift)
- existing product files on `develop`: `.sc-lint/bootstrap`,
  `.sc-lint/bootstrap.ps1`, the `sc-lint init --just` output

## Exact Targets

- `packages/sc-lint-adoption/install.py` (new)
- `packages/sc-lint-adoption/README.md` (new; installed as `README.sc-lint.md`)
- `packages/sc-lint-adoption/.claude-plugin/plugin.json` (new)
- `packages/sc-lint-adoption/.sc-lint/bootstrap` (new, verbatim from product)
- `packages/sc-lint-adoption/.sc-lint/bootstrap.ps1` (new, verbatim)
- `packages/sc-lint-adoption/.sc-lint/justfile` (new)
- `packages/sc-lint-adoption/.github/actions/setup-sc-lint/action.yml` (new)
- `packages/sc-lint-adoption/.github/workflows/sc-lint.yml` (new)
- `packages/sc-lint-adoption/templates/sc-lint.toml.j2` (new)
- `packages/sc-lint-adoption/templates/Justfile.import.j2` (new)
- `packages/sc-lint-adoption/install.schema.json` (new)
- `tests/adoption/test_install.py` (new)
- `tests/fixtures/adoption/empty-workspace/` (new)
- `tests/fixtures/adoption/established-workspace/` (new, synthetic)
- `tests/fixtures/adoption/install.json` (new)
- `.github/workflows/ci.yml` (add the adoption matrix job)
- `Justfile` (source-maintainer recipe `test-adoption`)

## Governing Contract

This sprint implements REQ-PRODUCT-019, REQ-PRODUCT-020,
REQ-PRODUCT-022, and REQ-PRODUCT-023. ADR-012 fixes the four-recipe consumer
surface; ADR-015 fixes kit ownership. `install.py` must not invent a second
consumer command contract or a source-checkout fallback.

## Recovered From Phase F

Recover by copying text from `sprint/F.4b-action-and-workflow-transformer`
into the files above; do not cherry-pick commits and do not port Rust.

- Justfile marker block signature and import line from
  `crates/sc-lint/src/configure/just.rs`:
  `# >>> sc-lint managed integration >>>` … `import '.sc-lint/justfile'` …
  `# <<< sc-lint managed integration <<<` → `templates/Justfile.import.j2`
- `CANONICAL_WORKFLOW` YAML from `crates/sc-lint/src/configure/workflow.rs`
  → `.github/workflows/sc-lint.yml`, with the `randlee/sc-lint@v1` action use
  replaced by the local `./.github/actions/setup-sc-lint`
- context/request JSON schema shape from `scripts/sc_lint_configure.py`
  lines 87–233 → `install.schema.json` (fields only; no planner logic)
- generic schema fixtures under `tests/fixtures/configure/contracts/` →
  `tests/fixtures/adoption/` where still meaningful

Explicitly discarded: `configure/{apply,artifact,legacy,reviewed_removals}.rs`,
`LEGACY_SC_COMPOSE_04`, the Wyvern driver, `docs/plans/phase-F/*`.

## Deliverables

- `install.py --input <install.json> <repo>` copies every kit file except
  `templates/`, `install.py`, `install.schema.json`, `README.md`, `.claude-plugin/`
  byte-for-byte; renames `README.md` → `README.sc-lint.md`; renders exactly
  `sc-lint.toml` from `templates/sc-lint.toml.j2`; inserts or replaces exactly
  one marker block in the root `Justfile` (creating `Justfile` if absent).
- `install.py --dry-run --input <install.json> <repo>` prints a unified diff
  per drifting file and exits 1 on drift, 0 when clean; writes nothing.
- A consumer-modified managed file, or a root `Justfile` containing zero or
  more than one marker block, is reported as a conflict with the path and
  exit 2; nothing is written. There is no delete operation.
- `install.schema.json` validates `install.json`; required fields:
  `minimum_version` (SemVer string), `profiles` (map name → argv list),
  `ci` (`{ os: [..], enabled: bool }`). Unknown fields are an error.
- `.sc-lint/justfile` defines `setup`, `lint`, `test`, `upgrade` each as one
  line delegating to `.sc-lint/bootstrap <op> --config sc-lint.toml` (POSIX)
  with the documented Windows shebang fallback to `bootstrap.ps1`.
- `setup-sc-lint/action.yml` reads `minimum_version` from `sc-lint.toml`
  (input `config`, default `sc-lint.toml`), downloads the matching release
  for Linux/macOS/Windows, verifies `sc-lint version --json`, and copies
  **nothing** from a source archive.
- Fixtures: `empty-workspace` (Cargo workspace, no Justfile) and
  `established-workspace` (synthetic Justfile with unrelated recipes, existing
  CI workflow, no consumer name). No fixture contains a real repository name.
- `plugin.json` with `name` `sc-lint-adoption`, `version` equal to the
  workspace version, `description`, `author`.

## Unblock Milestone

Commit the minimal, runnable kit interface G.2 documents:
`packages/sc-lint-adoption/install.py` accepts `--input`, `--dry-run`, and a
repository positional argument; `install.schema.json` validates its input;
the generic kit asset set and `tests/fixtures/adoption/install.json` let the
empty-workspace fixture install and then report a clean dry run. Report that
commit immediately; G.2 starts from it on `sprint/G.1-adoption-kit` while
G.1 completes conflict cases, the established fixture, CI matrix, and review.

## Acceptance Criteria

- `python3 packages/sc-lint-adoption/install.py --input tests/fixtures/adoption/install.json <tmp-empty>`
  exits 0; immediate `--dry-run` exits 0; `just --list` in `<tmp-empty>` shows
  exactly `setup lint test upgrade`.
- Same sequence on `<tmp-established>`: pre-existing recipes are unchanged
  byte-for-byte outside the marker block (`git diff --stat` shows only the
  block insertion).
- Editing `.sc-lint/justfile` in a consumer then `--dry-run` → exit 1 with a
  diff naming that file.
- Duplicating the marker block then running install → exit 2, no file written.
- `grep -rEn "sc-compose|atm-core|wyvern" packages/ tests/adoption tests/fixtures/adoption` returns nothing.
- CI job `adoption` passes on `ubuntu-latest`, `macos-latest`, `windows-latest`.
- `jq -e '.name == "sc-lint-adoption" and .version != "" and .description != "" and .author != ""' packages/sc-lint-adoption/.claude-plugin/plugin.json` exits 0.

## Required Validation

- `python3 -m pytest tests/adoption -q`
- `just test-adoption`
- `cargo test --workspace` (no crate changes expected; must stay green)
- `sc-lint lint --profile ci` on this repository

## Out Of Scope

- the adoption skill and agent prompts (G.2)
- changes to `crates/*` (G.3)
- any consumer repository change (G.4a–G.4c)
