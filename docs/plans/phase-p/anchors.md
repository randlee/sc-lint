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

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 2.

Summary: Ownership table covers every path

Scope: changes consumer_integration.rs, ADR-023, or the set of paths init writes or deletes

Deterministic check to pair with the development gate: every path in OWNED_ASSETS, OWNED_DIRECTORIES, the create-if-missing list and RETIRED_PATHS appears in ADR-023 sections 2 and 3, and vice versa

Fixtures:
- pass: Step 3: in `consumer_integration.rs`, add `.sc-lint/hooks/pre-commit` to OWNED_ASSETS and add the same path to the ADR-023 section 2 ownership table as sc-lint-owned, so every path init writes or deletes stays listed in the table.
- fail: Step 3: in `consumer_integration.rs`, add `.sc-lint/hooks/pre-commit` to OWNED_ASSETS so init writes it into consumers; ADR-023 is not changed and its ownership table does not list the new path.

## ANC-002

Anchor: `sc-lint init --just` must write every sc-lint-owned file with the bytes embedded in the binary, without reading, hashing, or comparing the existing file.

> sc-lint should simply overwrite or remove files. why are we doing binary compare, that seems excessive.  The issue is there should be no 'maybe overwrite' files.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 2.

Summary: Owned files always overwritten, no compare

Scope: changes how init writes sc-lint-owned files

Deterministic check to pair with the development gate: rg -n 'sha2|digest|KNOWN_PRIOR|INTEGRATION_CONFLICT|OUTDATED' crates/sc-lint/src/consumer_integration.rs returns nothing; a test edits an owned file and init restores the embedded bytes

Fixtures:
- pass: Step 1: in `consumer_integration.rs`, init writes each sc-lint-owned file with `fs::write(path, EMBEDDED_BYTES)` unconditionally; it does not read, hash or compare the existing file first.
- fail: Step 1: in `consumer_integration.rs`, init computes the sha256 of each existing sc-lint-owned file and overwrites it only when the hash differs from the embedded bytes; otherwise it skips the write.

## ANC-003

Anchor: `sc-lint init --just` never modifies an existing `sc-lint.toml`, `Justfile`, `justfile`, or `.github/workflows/*` file in a consumer repository.

> there are sc-lint owned files and consumer owned files that are well defined.  that is what these sprints do.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 2.

Summary: Consumer files created only if missing

Scope: changes how init treats consumer-owned files

Deterministic check to pair with the development gate: a test edits sc-lint.toml and Justfile, runs init, and asserts both are byte-identical afterwards

Fixtures:
- pass: Step 4: init creates `sc-lint.toml` and `Justfile` only when they are missing; if either file, or any `.github/workflows/*` file, already exists in the consumer, init leaves it byte-for-byte unchanged.
- fail: Step 4: init appends an `import 'sc-lint.just'` line to the consumer's existing `Justfile` when that line is absent.

## ANC-004

Anchor: `sc-lint init --just` must delete each ADR-023 retired path present in the consumer: `plugins/sc-lint/`, `.sc-lint/install.json`, `.sc-lint/installed.json`, `.sc-lint/source-install.json`, `README.sc-lint.md`.

> sc-lint needs to delete retired paths.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 2.

Summary: init deletes retired paths

Scope: changes init behavior for retired paths or the retired-path list

Deterministic check to pair with the development gate: a test seeds every retired path, runs init, and asserts none remain; --dry-run removes nothing

Fixtures:
- pass: Step 5: init deletes each retired path present in the consumer: `plugins/sc-lint/`, `.sc-lint/install.json`, `.sc-lint/installed.json`, `.sc-lint/source-install.json` and `README.sc-lint.md`.
- fail: Step 5: init leaves retired paths such as `plugins/sc-lint/` and `.sc-lint/installed.json` in place and prints a warning asking the consumer to delete them by hand.

## ANC-005

Anchor: sc-lint ships no `init --check` drift-detection mode; what `sc-lint init --just` changed is reviewed through the consumer's commit or pull-request diff.

> yes, and when sc-lint is installed it any overwrites/deletes will show up when commit/pr

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 2.

Summary: Review is the commit/PR diff; no init --check

Scope: adds, keeps, or documents an init check or drift mode

Deterministic check to pair with the development gate: sc-lint init --just --check is rejected by the CLI parser; rg -n 'init --just --check' docs docs-bundle crates returns nothing

Fixtures:
- pass: Step 2: `docs-bundle/configuration.md` tells consumers to review what `sc-lint init --just` changed through their commit or pull-request diff; no `init --check` mode is added.
- fail: Step 2: add an `sc-lint init --just --check` flag that reports drift between the consumer's files and the embedded copies and exits non-zero in CI.

## ANC-006

Anchor: Before 0.8.7 is published, `docs/migrations/0.8.7/` holds one guide each for atm-core, sc-compose and sc-observability listing every file the upgrade deletes or breaks there and its fix.

> the migration guide forces us to forcee what will break before delivering.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 2.

Summary: Per-consumer migration guide before release

Scope: writes or changes migration guides, or the release step for 0.8.7

Deterministic check to pair with the development gate: ls docs/migrations/0.8.7/{atm-core,sc-compose,sc-observability}.md

Fixtures:
- pass: Step 1: before 0.8.7 is published, write `docs/migrations/0.8.7/atm-core.md`, `sc-compose.md` and `sc-observability.md`, each listing every file the upgrade deletes or breaks in that repo with its fix.
- fail: Step 1: write one `docs/migrations/0.8.7/README.md` with generic upgrade steps shared by all consumers; no per-repo guide for atm-core, sc-compose or sc-observability is written.

## ANC-007

Anchor: The atm-core migration guide must cite files as `path@sha` on `integrate/phase-bg`, and the sc-compose guide must cite files as `path@sha` on `integrate/phase-v`.

> yes, I would like a task on this phase to review atm-core (integrate/phase-bg), sc-compose (integrate/phase-v), sc-observability and write up migration guides for those 3 repos specifically.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 2.

Summary: Guides cite the named consumer branches

Scope: writes or reviews the atm-core or sc-compose migration guide

Deterministic check to pair with the development gate: every path@sha in each guide resolves via gh api repos/randlee/<repo>/contents/<path>?ref=<sha>

Fixtures:
- pass: Step 2: in `docs/migrations/0.8.7/atm-core.md`, cite every affected file as `path@sha` on atm-core `integrate/phase-bg`; in `sc-compose.md`, cite every affected file as `path@sha` on sc-compose `integrate/phase-v`.
- fail: Step 2: in `docs/migrations/0.8.7/atm-core.md` and `sc-compose.md`, cite every affected file as `path@sha` on each repo's `main` branch.

## ANC-008

Anchor: The atm-core and sc-compose migration guides must classify the failure on each listed prior failed sc-lint upgrade branch as file ownership, sc-lint bug, or consumer-side.

> both repos have branches they have continuously tried to upgrade to sc-lint and failed.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 2.

Summary: Failed upgrade branches classified

Scope: writes or reviews the atm-core or sc-compose migration guide

Deterministic check to pair with the development gate: each branch named in the guide's failed-branch list has a classification row

Fixtures:
- pass: Step 3: in the atm-core and sc-compose migration guides, add a failed-upgrade table with one row per prior failed sc-lint upgrade branch, classifying its failure as file ownership, sc-lint bug, or consumer-side.
- fail: Step 3: in the atm-core and sc-compose migration guides, list the prior failed sc-lint upgrade branches by name and link each one to its last CI run.

## ANC-009

Anchor: A phase-p sprint must run the documented upgrade with the 0.8.7 candidate in unpushed worktrees of atm-core `integrate/phase-bg` and sc-compose `integrate/phase-v` and record `just setup`, `just lint`, `just test` results.

> you can run the upgrade on a /sc-git-worktree on the atm-core and sc-compose repos (off the branch I specified).

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 3.

Summary: Real upgrade run in consumer worktrees

Scope: plans or performs the consumer upgrade run

Deterministic check to pair with the development gate: the guide records the commands and exit codes of the run for each repo

Fixtures:
- pass: Step 4: create unpushed worktrees of atm-core `integrate/phase-bg` and sc-compose `integrate/phase-v`, run the documented upgrade with the 0.8.7 candidate in each, and record the results of `just setup`, `just lint` and `just test`.
- fail: Step 4: verify the documented upgrade by running it with the 0.8.7 candidate in a fixture repository under `tests/fixtures/consumer/` and recording `just setup`, `just lint` and `just test`; atm-core and sc-compose are not used.

## ANC-010

Anchor: 0.8.7 is not published until `just setup`, `just lint` and `just test` pass in atm-core and sc-compose with the release candidate installed.

> I do NOT want another round of issues.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 2.

Summary: Consumer canary gates publish

Scope: changes the release workflow, the canary, or release preflight

Deterministic check to pair with the development gate: the publish job depends on a canary job that runs just setup/lint/test in atm-core and sc-compose

Fixtures:
- pass: Step 2: in the release workflow, make the publish job depend on a canary job that installs the 0.8.7 candidate in atm-core and sc-compose and requires `just setup`, `just lint` and `just test` to pass.
- fail: Step 2: in the release workflow, publish 0.8.7 as soon as sc-lint's own CI passes; the atm-core and sc-compose canary runs after publish and only reports its results.

## ANC-011

Anchor: After phase-p, `sc-lint init --just` is the only code that writes sc-lint files into a consumer repository; `install.py` and every other installer copy are deleted.

> we can not leave stale scripts to support one method and not the other.

Said by Rand, phase-o planning; Rand's direction recorded verbatim in ADR-023 (2026-11-05); locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-05. Status: locked. Version 3.

Summary: One write path into consumers

Scope: adds, keeps, or removes code that writes sc-lint files into a consumer

Deterministic check to pair with the development gate: git ls-files | rg 'install\.py$|install\.schema\.json' returns nothing under packages/sc-lint-adoption

Fixtures:
- pass: Step 3: delete `packages/sc-lint-adoption/install.py` and `install.schema.json`; `sc-lint init --just` remains the only code that writes sc-lint files into a consumer.
- fail: Step 3: keep `packages/sc-lint-adoption/install.py` as a fallback installer for consumers without the sc-lint binary, and update it to copy the same files as init.

## ANC-012

Anchor: sc-lint ships no code that inspects a consumer's existing files to infer what to change; `sc-lint init --just` acts only on fixed path lists.

> I do not want more scripts that "guess" about what to do.

Said by Rand, phase-o planning; Rand's direction recorded verbatim in ADR-023 (2026-11-05); locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-05. Status: locked. Version 2.

Summary: No state-guessing scripts

Scope: adds detection, migration, or inference logic to init, bootstrap, or setup

Deterministic check to pair with the development gate: init's write and delete sets are compile-time constants; no branch depends on existing file content

Fixtures:
- pass: Step 2: init deletes the paths in the RETIRED_PATHS constant and writes the paths in OWNED_ASSETS; it never reads a consumer file's contents to decide what to change.
- fail: Step 2: init reads the consumer's existing `Justfile` and `.sc-lint/` directory to detect which older sc-lint version was installed, then runs the matching migration steps.

## ANC-013

Anchor: `docs-bundle/configuration.md` must list every path sc-lint writes in a consumer and mark each as sc-lint-owned, consumer-owned, or retired.

> There must be clear documented config files and what we "upgrade" and what customers use for config must be clearly documented.

Said by Rand, phase-o planning; Rand's direction recorded verbatim in ADR-023 (2026-11-05); locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-05. Status: locked. Version 3.

Summary: Ownership documented for consumers

Scope: changes consumer-facing docs or the ownership table

Deterministic check to pair with the development gate: every ADR-023 ownership and retired path appears in docs-bundle/configuration.md

Fixtures:
- pass: Step 1: in `docs-bundle/configuration.md`, add a table listing every path sc-lint writes in a consumer, marking each as sc-lint-owned, consumer-owned or retired.
- fail: Step 1: in `docs-bundle/configuration.md`, add a table of the consumer-owned files (`sc-lint.toml`, `Justfile`); the sc-lint-owned files init overwrites and the retired paths are left out of the table.

## ANC-014

Anchor: Phase-p must delete more lines than it adds across `bootstrap.py`, `consumer_integration.rs`, `setup_tools.py` and `packages/sc-lint-adoption/`.

> I do NOT want a lot of code to install sc-lint.

Said by Rand, phase-o epic provenance (Rand 2026-11-05); locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-05. Status: locked. Version 3.

Summary: Less install code

Scope: every phase-p sprint that touches the install path

Deterministic check to pair with the development gate: git diff --shortstat origin/develop...integrate/phase-p -- <those paths> shows deletions > insertions

Fixtures:
- pass: Step 5: delete the legacy file-copy logic in `bootstrap.py` and `setup_tools.py` (about 400 lines) and the whole `packages/sc-lint-adoption/` installer, replacing them with a 20-line call to `sc-lint init --just`.
- fail: Step 5: add a new 300-line repair module to `setup_tools.py` that re-validates each consumer install, keeping the existing `bootstrap.py` copy logic unchanged.

## ANC-015

Anchor: Every phase-p sprint bead must list numbered steps that each name a file path and its exact change.

> For now, I want specific tasks written up w/ code details (low on prose) so two agents would do the same thing if prompted.

Said by Rand, phase-o planning, Flint session b1876c4f; locked by Rand 2026-11-07 ('anchors are locked'), 2026-11-07. Status: locked. Version 3.

Summary: Beads are numbered file-level steps

Scope: every phase-p sprint bead

Deterministic check to pair with the development gate: each sprint bead has a numbered step list; every step names a path

Fixtures:
- pass: 1. `crates/sc-lint/src/consumer_integration.rs`: remove the `sha2` import and the `KNOWN_PRIOR` table. 2. `docs/adr/ADR-023.md`: add `README.sc-lint.md` to the section 3 retired list.
- fail: Clean up the install path so file ownership is clearer, simplify the integration code where it makes sense, and update the docs to match.
