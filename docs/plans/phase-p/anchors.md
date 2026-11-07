# Anchors - phase-p

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | Ownership table covers every path | 2026-11-07 |
| ANC-002 | locked | Owned files always overwritten, no compare | 2026-11-07 |
| ANC-003 | locked | Consumer files created only if missing | 2026-11-07 |
| ANC-004 | locked | init deletes retired paths | 2026-11-07 |
| ANC-005 | locked | Review is the commit/PR diff; no init --check | 2026-11-07 |
| ANC-006 | locked | Per-consumer migration guide before release | 2026-11-07 |
| ANC-007 | locked | Guides cite the named consumer branches | 2026-11-07 |
| ANC-008 | locked | Failed upgrade branches classified | 2026-11-07 |
| ANC-009 | locked | Real upgrade run in consumer worktrees | 2026-11-07 |
| ANC-010 | locked | Consumer canary gates publish | 2026-11-07 |
| ANC-011 | locked | One write path into consumers | 2026-11-05 |
| ANC-012 | locked | No state-guessing scripts | 2026-11-05 |
| ANC-013 | locked | Ownership documented for consumers | 2026-11-05 |
| ANC-014 | locked | Less install code | 2026-11-05 |
| ANC-015 | locked | Beads are numbered file-level steps | 2026-11-07 |

## ANC-001

Anchor: Every path that `sc-lint init --just` writes or deletes in a consumer repository is listed in the ADR-023 ownership table as sc-lint-owned, consumer-owned, or retired.

> there are sc-lint owned files and consumer owned files that are well defined.  that is what these sprints do.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: Ownership table covers every path

Scope: changes consumer_integration.rs, ADR-023, or the set of paths init writes or deletes

Deterministic check to pair with the development gate: every path in OWNED_ASSETS, OWNED_DIRECTORIES, the create-if-missing list and RETIRED_PATHS appears in ADR-023 sections 2 and 3, and vice versa

## ANC-002

Anchor: `sc-lint init --just` must write every sc-lint-owned file with the bytes embedded in the binary, without reading, hashing, or comparing the existing file.

> sc-lint should simply overwrite or remove files. why are we doing binary compare, that seems excessive.  The issue is there should be no 'maybe overwrite' files.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: Owned files always overwritten, no compare

Scope: changes how init writes sc-lint-owned files

Deterministic check to pair with the development gate: rg -n 'sha2|digest|KNOWN_PRIOR|INTEGRATION_CONFLICT|OUTDATED' crates/sc-lint/src/consumer_integration.rs returns nothing; a test edits an owned file and init restores the embedded bytes

## ANC-003

Anchor: `sc-lint init --just` never modifies an existing `sc-lint.toml`, `Justfile`, `justfile`, or `.github/workflows/*` file in a consumer repository.

> there are sc-lint owned files and consumer owned files that are well defined.  that is what these sprints do.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: Consumer files created only if missing

Scope: changes how init treats consumer-owned files

Deterministic check to pair with the development gate: a test edits sc-lint.toml and Justfile, runs init, and asserts both are byte-identical afterwards

## ANC-004

Anchor: `sc-lint init --just` must delete each ADR-023 retired path present in the consumer: `plugins/sc-lint/`, `.sc-lint/install.json`, `.sc-lint/installed.json`, `.sc-lint/source-install.json`, `README.sc-lint.md`.

> sc-lint needs to delete retired paths.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: init deletes retired paths

Scope: changes init behavior for retired paths or the retired-path list

Deterministic check to pair with the development gate: a test seeds every retired path, runs init, and asserts none remain; --dry-run removes nothing

## ANC-005

Anchor: sc-lint ships no `init --check` drift-detection mode; what `sc-lint init --just` changed is reviewed through the consumer's commit or pull-request diff.

> yes, and when sc-lint is installed it any overwrites/deletes will show up when commit/pr

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: Review is the commit/PR diff; no init --check

Scope: adds, keeps, or documents an init check or drift mode

Deterministic check to pair with the development gate: sc-lint init --just --check is rejected by the CLI parser; rg -n 'init --just --check' docs docs-bundle crates returns nothing

## ANC-006

Anchor: Before 0.8.7 is published, `docs/migrations/0.8.7/` holds one guide each for atm-core, sc-compose and sc-observability listing every file the upgrade deletes or breaks there and its fix.

> the migration guide forces us to forcee what will break before delivering.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: Per-consumer migration guide before release

Scope: writes or changes migration guides, or the release step for 0.8.7

Deterministic check to pair with the development gate: ls docs/migrations/0.8.7/{atm-core,sc-compose,sc-observability}.md

## ANC-007

Anchor: The atm-core migration guide must cite files as `path@sha` on `integrate/phase-bg`, and the sc-compose guide must cite files as `path@sha` on `integrate/phase-v`.

> yes, I would like a task on this phase to review atm-core (integrate/phase-bg), sc-compose (integrate/phase-v), sc-observability and write up migration guides for those 3 repos specifically.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: Guides cite the named consumer branches

Scope: writes or reviews the atm-core or sc-compose migration guide

Deterministic check to pair with the development gate: every path@sha in each guide resolves via gh api repos/randlee/<repo>/contents/<path>?ref=<sha>

## ANC-008

Anchor: The atm-core and sc-compose migration guides must classify the failure on each listed prior failed sc-lint upgrade branch as file ownership, sc-lint bug, or consumer-side.

> both repos have branches they have continuously tried to upgrade to sc-lint and failed.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: Failed upgrade branches classified

Scope: writes or reviews the atm-core or sc-compose migration guide

Deterministic check to pair with the development gate: each branch named in the guide's failed-branch list has a classification row

## ANC-009

Anchor: A phase-o sprint must run the documented upgrade with the 0.8.7 candidate in unpushed worktrees of atm-core `integrate/phase-bg` and sc-compose `integrate/phase-v` and record `just setup`, `just lint`, `just test` results.

> you can run the upgrade on a /sc-git-worktree on the atm-core and sc-compose repos (off the branch I specified).

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: Real upgrade run in consumer worktrees

Scope: plans or performs the consumer upgrade run

Deterministic check to pair with the development gate: the guide records the commands and exit codes of the run for each repo

## ANC-010

Anchor: 0.8.7 is not published until `just setup`, `just lint` and `just test` pass in atm-core and sc-compose with the release candidate installed.

> I do NOT want another round of issues.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: Consumer canary gates publish

Scope: changes the release workflow, the canary, or release preflight

Deterministic check to pair with the development gate: the publish job depends on a canary job that runs just setup/lint/test in atm-core and sc-compose

## ANC-011

Anchor: After phase-o, `sc-lint init --just` is the only code that writes sc-lint files into a consumer repository; `install.py` and every other installer copy are deleted.

> we can not leave stale scripts to support one method and not the other.

Said by Rand, phase-o planning; Rand's direction recorded verbatim in ADR-023 (2026-11-05); locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-05. Status: locked. Version 1.

Summary: One write path into consumers

Scope: adds, keeps, or removes code that writes sc-lint files into a consumer

Deterministic check to pair with the development gate: git ls-files | rg 'install\.py$|install\.schema\.json' returns nothing under packages/sc-lint-adoption

## ANC-012

Anchor: sc-lint ships no code that inspects a consumer's existing files to infer what to change; `sc-lint init --just` acts only on fixed path lists.

> I do not want more scripts that "guess" about what to do.

Said by Rand, phase-o planning; Rand's direction recorded verbatim in ADR-023 (2026-11-05); locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-05. Status: locked. Version 1.

Summary: No state-guessing scripts

Scope: adds detection, migration, or inference logic to init, bootstrap, or setup

Deterministic check to pair with the development gate: init's write and delete sets are compile-time constants; no branch depends on existing file content

## ANC-013

Anchor: `docs-bundle/configuration.md` must list every path sc-lint writes in a consumer and mark each as sc-lint-owned, consumer-owned, or retired.

> There must be clear documented config files and what we "upgrade" and what customers use for config must be clearly documented.

Said by Rand, phase-o planning; Rand's direction recorded verbatim in ADR-023 (2026-11-05); locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-05. Status: locked. Version 1.

Summary: Ownership documented for consumers

Scope: changes consumer-facing docs or the ownership table

Deterministic check to pair with the development gate: every ADR-023 ownership and retired path appears in docs-bundle/configuration.md

## ANC-014

Anchor: Phase-o must delete more lines than it adds across `bootstrap.py`, `consumer_integration.rs`, `setup_tools.py` and `packages/sc-lint-adoption/`.

> I do NOT want a lot of code to install sc-lint.

Said by Rand, phase-o epic provenance (Rand 2026-11-05); locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-05. Status: locked. Version 1.

Summary: Less install code

Scope: every phase-o sprint that touches the install path

Deterministic check to pair with the development gate: git diff --shortstat origin/develop...integrate/phase-o -- <those paths> shows deletions > insertions

## ANC-015

Anchor: Every phase-o sprint bead must list numbered steps that each name a file path and its exact change.

> For now, I want specific tasks written up w/ code details (low on prose) so two agents would do the same thing if prompted.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 1.

Summary: Beads are numbered file-level steps

Scope: every phase-o sprint bead

Deterministic check to pair with the development gate: each sprint bead has a numbered step list; every step names a path
