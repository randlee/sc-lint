# Anchors - phase-m

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-m/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | retired | A1 #786: PORT-010 per-shape classification of the 18 atm-core paths | 2026-10-30 |
| ANC-002 | retired | A2 #720: PORT-010 crate-root cfg(unix)/cfg(not(unix)) companion | 2026-10-30 |
| ANC-003 | retired | A3 #198: consumer setup.pre hook (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving).) | 2026-10-30 |
| ANC-004 | retired | A4 #198: consumer runner.python (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving).) | 2026-10-30 |
| ANC-005 | retired | A5 #198: wheel-only sc_lint helpers on the kit interpreter (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving). Design also awaits atm-core input.) | 2026-10-30 |
| ANC-006 | retired | A6 #136: source-only scope for global_dependency_ownership | 2026-10-30 |
| ANC-007 | retired | A7 #136: three-key parity audit | 2026-10-30 |
| ANC-008 | retired | #786/#720: no PORT-010 false alarms on atm-core's excluded code | 2026-10-30 |
| ANC-009 | retired | #786/#720: real Windows gaps still reported | 2026-10-30 |
| ANC-010 | proposed | #198: sc-lint runs the consumer setup step | 2026-10-30 |
| ANC-011 | proposed | #198: sc-lint uses the right Python itself | 2026-10-30 |
| ANC-012 | proposed | #136: dependency ownership on source files only | 2026-10-30 |
| ANC-013 | retired | #136: record coverage of atm-core's three boundary settings | 2026-10-30 |
| GH-786.table | retired | #786 table: none of the 47 findings reported | 2026-10-30 |
| GH-786.S3 | proposed | #720/S3: same-name cfg(not(unix)) sibling, all scopes | 2026-10-30 |
| GH-198.setup-pre | retired | #198: [tool.sc-lint].setup.pre | 2026-10-30 |
| GH-198.runner-argv | proposed | #198: runner.python resolves leading python, lint and test | 2026-10-30 |
| GH-198.runner-path | retired | #198: runner.python venv dir prepended to child PATH | 2026-10-30 |
| GH-198.kit-helpers | proposed | #198: sc_lint.* helpers on the kit interpreter | 2026-10-30 |
| GH-198.export-env | retired | #198: export PYO3_PYTHON for native steps (Rand scope ruling needed) | 2026-10-30 |
| GH-136.src-only | retired | #136: source-only scope setting for global_dependency_ownership | 2026-10-30 |
| GH-136.audit | retired | #136: audit the three atm-core keys, file gaps | 2026-10-30 |
| GH-786.table.2 | retired | #786: none of the 47 findings reported, exclusions removable | 2026-10-30 |
| GH-198.setup-pre.2 | proposed | #198: [tool.sc-lint].setup.pre runs before provisioning | 2026-10-30 |
| GH-198.runner-path.2 | proposed | #198: runner.python venv dir prepended to every step's PATH | 2026-10-30 |
| GH-198.export-env.2 | retired | #198: export PYO3_PYTHON to native steps (Rand scope ruling needed) | 2026-10-30 |
| GH-136.src-only.2 | proposed | #136: source-only scope setting for global_dependency_ownership | 2026-10-30 |
| GH-136.audit.2 | retired | #136: audit the three atm-core keys, file gaps | 2026-10-30 |
| GH-136.audit-verify | proposed | #136 audit: verify coverage of the three keys | 2026-10-30 |
| GH-136.audit-gaps | proposed | #136 audit: file a gap per uncovered key | 2026-10-30 |
| ANC-014 | proposed | #786/#720: no PORT-010 false alarms on atm-core | 2026-10-30 |
| ANC-015 | proposed | #786/#720: real Windows gaps still reported | 2026-10-30 |
| ANC-016 | proposed | #136: parity table of atm-core's three boundary settings | 2026-10-30 |
| ANC-017 | proposed | #786 S8: pub POSIX-only helpers exempt | 2026-10-30 |
| GH-786.table.3 | proposed | #786: none of the 47 findings reported, exclusions removable | 2026-10-30 |
| GH-198.export-env.3 | proposed | #198: export PYO3_PYTHON to native steps | 2026-10-30 |

## ANC-001

Anchor: Each PORT-010 finding in sc-lint#786 must be resolved by shape classification: legitimate platform adapter or helper shapes stop being reported, and real cross-platform parity gaps are still reported.

> these are the p1 items from atm-core: - randlee/sc-lint#786 (PORT-010): all 18 excluded atm-core paths still emit 1–17 findings on 0.8.2, so I removed no exclusions. sc-lint.toml cites it with a release-based removal condition. This is the one that keeps atm-core exclusions in place.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; acceptance text from sc-lint#786 'Requested outcome' at atm-core d70ff224, 2026-10-30. Status: retired. Version 3.

Summary: A1 #786: PORT-010 per-shape classification of the 18 atm-core paths

Scope: The item changes PORT-010 in crates/sc-lint-portability, adds or edits port010 fixtures, edits PORT-010 docs, or changes how atm-core PORT-010 exclusions or platform_adapters are handled.

Deterministic check to pair with the development gate: One regression fixture per classified shape under crates/sc-lint-portability/tests/fixtures/port010/ with both an exempt and a real-gap case; the #786 per-path repro (remove one exclude, run sc-lint --json lint sc-portability) on atm-core d70ff224 reports only shapes classified as real gaps.

Fixtures:
- pass: Adds one port010 fixture per classified #786 shape; the exempt adapter cases report nothing and the real-gap cases still report PORT-010.
- fail: Adds the 18 atm-core paths to a default PORT-010 exclusion list so the #786 repro reports nothing.

## ANC-002

Anchor: PORT-010 must give a crate-root #[cfg(unix)] fn with a #[cfg(not(unix))] sibling in the same file one documented classification, pinned by a fixture of atm-core's crates/atm/src/main.rs shape.

> these are the p1 items from atm-core: - randlee/sc-lint#720 (PORT-010): a crate-root cfg(unix) fn with a cfg(not(unix)) sibling is still reported. It covers only main.rs.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; acceptance criteria from sc-lint#720, 2026-10-30. Status: retired. Version 3.

Summary: A2 #720: PORT-010 crate-root cfg(unix)/cfg(not(unix)) companion

Scope: The item changes PORT-010 sibling/companion detection, edits the port010 crate_root_main fixture or port010_fixture_crate_root_main_current_behaviour, or edits PORT-010 docs.

Deterministic check to pair with the development gate: port010_fixture_crate_root_main_current_behaviour (crates/sc-lint-portability/tests/fixtures/port010/crate_root_main) is replaced by a test pinning the decided classification, a real-gap variant still reports, and the PORT-010 docs diff states the classification.

Fixtures:
- pass: Replaces port010_fixture_crate_root_main_current_behaviour with a test pinning the decided classification and documents it in the PORT-010 docs.
- fail: Stops reporting the crate-root main.rs shape with no fixture change and no PORT-010 docs update.

## ANC-003

Anchor: The [tool.sc-lint] consumer config must accept setup.pre, an argv list that just setup runs before sc-lint provisioning.

> these are the p1 items from atm-core: - randlee/sc-lint#198 (consumer setup hook and step runner): atm-core keeps _py, sc_lint_clippy_prefix and _sc-lint-clippy until it ships.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; proposal text from sc-lint#198. CONFLICT: ADR-020 decisions 9 and 13 (Accepted, Rand 2026-10-27) say setup.pre is not added and root-level setup is rejected with config.unknown_table, 2026-10-30. Status: retired. Version 4.

Summary: A3 #198: consumer setup.pre hook (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving).)

Scope: The item edits ScLintConfigFile or consumer config parsing in crates/sc-lint (config.rs), the just setup / installer flow, config.unknown_table handling, or ADR-016/ADR-020 hook decisions.

Deterministic check to pair with the development gate: A config-parse test accepts setup.pre; a setup test with a stub command records that it ran before provisioning; ADR-020 decisions 9 and 13 are amended.

Fixtures:
- pass: Adds [tool.sc-lint].setup.pre to ScLintConfigFile; just setup runs the configured argv before provisioning, shown by a stub-command ordering test.
- fail: Keeps rejecting [tool.sc-lint].setup with config.unknown_table, so consumers still need Justfile glue before just setup.

## ANC-004

Anchor: The [tool.sc-lint] consumer config must accept runner.python, a venv path whose interpreter replaces a leading python in each lint and test step argv on Linux, macOS and Windows.

> these are the p1 items from atm-core: - randlee/sc-lint#198 (consumer setup hook and step runner): atm-core keeps _py, sc_lint_clippy_prefix and _sc-lint-clippy until it ships.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; proposal text from sc-lint#198 and atm-core comment 2026-10-29 item 2. CONFLICT: ADR-020 decisions 9 and 13 (Accepted, Rand 2026-10-27) say runner.python is not added, 2026-10-30. Status: retired. Version 4.

Summary: A4 #198: consumer runner.python (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving).)

Scope: The item edits ScLintConfigFile or consumer config parsing, lint/test step argv construction in crates/sc-lint, or venv interpreter path resolution (bin/python vs Scripts\python.exe).

Deterministic check to pair with the development gate: Deterministic argv-rewrite unit tests over Unix and Windows venv path forms (bin/python, Scripts\python.exe) with injected platform, no real interpreter.

Fixtures:
- pass: Adds runner.python; a step argv starting with python runs the venv's bin/python on Unix and Scripts\python.exe on Windows, covered by injected-platform tests.
- fail: Runs a step's leading python from PATH regardless of the configured runner.python venv.

## ANC-005

Anchor: A consumer step must run a wheel-only sc_lint.<module> helper on the sc-lint kit's own interpreter without the consumer writing a per-OS interpreter path.

> these are the p1 items from atm-core: - randlee/sc-lint#198 (consumer setup hook and step runner): atm-core keeps _py, sc_lint_clippy_prefix and _sc-lint-clippy until it ships.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; need stated in atm-core comment on sc-lint#198 2026-10-29 item 1 (no step form to run a wheel-only module on the kit interpreter). Design undecided; awaits atm-core input, 2026-10-30. Status: retired. Version 3.

Summary: A5 #198: wheel-only sc_lint helpers on the kit interpreter (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving). Design also awaits atm-core input.)

Scope: The item adds a step kind or subcommand for running sc_lint modules, edits consumer step execution in crates/sc-lint, or documents invoking .sc-lint/venv helpers.

Deterministic check to pair with the development gate: A runner test with a stub kit interpreter on Unix and Windows path forms shows a module step resolves to the kit venv interpreter; consumer config for it contains no OS-specific path.

Fixtures:
- pass: Adds a module step kind so {"module": "sc_lint.lint_cargo_deny"} runs on the kit venv interpreter on every OS.
- fail: Documents that consumers must call .sc-lint/venv/bin/python or Scripts\python.exe themselves for each OS.

## ANC-006

Anchor: global_dependency_ownership must offer a source-only scope in which SCB-MANIFEST-004 skips integration-test targets under crates/*/tests/ but still reports imports in production src files.

> these are the p1 items from atm-core: - randlee/sc-lint#136 (manifest dependency allowlists and global ownership versus the released manifest_policy): documented as a gap in the #2295 body.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; repro from atm-core comment on sc-lint#136 2026-10-29 (eight SCB-MANIFEST-004 findings on 0.8.2), 2026-10-30. Status: retired. Version 3.

Summary: A6 #136: source-only scope for global_dependency_ownership

Scope: The item edits global_dependency_ownership or SCB-MANIFEST-004 in crates/sc-lint-boundary (manifest_dependency_policy), its config keys, or docs-bundle/packages/sc-lint-boundary.md.

Deterministic check to pair with the development gate: A boundary test reproducing the #136 tree: the seven crates/*/tests/ findings disappear under the source-only scope and crates/atm/src/commands/mod.rs importing cron is still reported; no allowed_source_roots or excludes added.

Fixtures:
- pass: Adds a source-only ownership scope; the #136 repro drops the seven crates/*/tests/ findings and still reports crates/atm/src/commands/mod.rs importing cron.
- fail: Tells atm-core to add its tests/ directories to allowed_source_roots or excludes to silence the findings.

## ANC-007

Anchor: Phase M must classify each atm-core [boundaries] key (manifest_dependency_allowlists, global_dependency_ownership, manifest_section_rules) as covered or not by released sc-lint-boundary, filing an issue per gap.

> these are the p1 items from atm-core: - randlee/sc-lint#136 (manifest dependency allowlists and global ownership versus the released manifest_policy): documented as a gap in the #2295 body.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; audit scope from sc-lint#136 body and team-lead A7 definition (2026-10-30), 2026-10-30. Status: retired. Version 3.

Summary: A7 #136: three-key parity audit

Scope: The item audits or documents atm-core [boundaries] key parity, writes a parity table under docs/plans/phase-m/, or files sc-lint-boundary gap issues.

Deterministic check to pair with the development gate: A parity table under docs/plans/phase-m/ has one row per key with covered/gap and evidence; each gap row links a GitHub issue.

Fixtures:
- pass: Writes a parity table with one row per atm-core [boundaries] key, citing evidence, and links a filed GitHub issue for each gap row.
- fail: Declares #136 resolved because SCB-MANIFEST-003..006 shipped, without checking each key against atm-core behaviour.

## ANC-008

Anchor: With atm-core's PORT-010 exclusions removed, PORT-010 must report zero false alarms on atm-core.

> atm-core can remove its PORT-010 exclusions and get zero false alarms, while real Windows gaps are still reported.

Said by Rand, Phase M planning; Rand approved this sentence as his words ('yes'), relayed by team-lead 2026-10-30, 2026-10-30. Status: retired. Version 3.

Summary: #786/#720: no PORT-010 false alarms on atm-core's excluded code

Scope: The item changes PORT-010 in crates/sc-lint-portability, its fixtures or docs, or atm-core PORT-010 exclusions.

Deterministic check to pair with the development gate: Per-path #786 repro on atm-core with every PORT-010 exclusion removed reports no finding classified false positive or deliberate exception.

Fixtures:
- pass: With all atm-core PORT-010 exclusions removed, the #786 repro reports no false alarm.
- fail: PORT-010 still reports atm-core's same-name cfg(not(unix)) siblings as missing Windows support.

## ANC-009

Anchor: PORT-010 must still report a real Windows gap: a cfg(unix) item with no Windows counterpart.

> atm-core can remove its PORT-010 exclusions and get zero false alarms, while real Windows gaps are still reported.

Said by Rand, Phase M planning; Rand approved this sentence as his words ('yes'), relayed by team-lead 2026-10-30, 2026-10-30. Status: retired. Version 3.

Summary: #786/#720: real Windows gaps still reported

Scope: The item changes PORT-010 exemptions, classification or suppression in crates/sc-lint-portability.

Deterministic check to pair with the development gate: A port010 fixture with a cfg(unix) fn and no Windows, cfg(not(unix)) or portable counterpart still reports PORT-010.

Fixtures:
- pass: Keeps a regression fixture where a cfg(unix) fn with no Windows counterpart still reports PORT-010.
- fail: Exempts every cfg(unix) fn in a file once any Windows code exists anywhere in that file.

## ANC-010

Anchor: sc-lint must run atm-core's setup step itself, so atm-core needs no Justfile glue to run it.

> atm-core can delete its Justfile glue (_py, sc_lint_clippy_prefix, _sc-lint-clippy) because sc-lint runs the setup step and uses the right Python itself.

Said by Rand, Phase M planning; Rand approved this sentence as his words ('yes'), relayed by team-lead 2026-10-30, 2026-10-30. Status: proposed. Version 4.

Summary: #198: sc-lint runs the consumer setup step

Scope: The item changes sc-lint setup/provisioning, consumer config ([tool.sc-lint].setup), or the just setup flow.

Deterministic check to pair with the development gate: A setup test with a stub configured command shows sc-lint runs it with no consumer Justfile recipe.

Fixtures:
- pass: sc-lint setup runs the consumer's configured setup step itself, so atm-core deletes its Justfile setup recipe. Evidence: test setup_runs_configured_step passes, showing the configured stub command ran during sc-lint setup with no Justfile recipe.
- fail: Documents that consumers keep a Justfile recipe to run their own setup before sc-lint.

## ANC-011

Anchor: sc-lint must choose the right Python interpreter for atm-core's steps itself, so atm-core can delete _py, sc_lint_clippy_prefix and _sc-lint-clippy.

> atm-core can delete its Justfile glue (_py, sc_lint_clippy_prefix, _sc-lint-clippy) because sc-lint runs the setup step and uses the right Python itself.

Said by Rand, Phase M planning; Rand approved this sentence as his words ('yes'), relayed by team-lead 2026-10-30, 2026-10-30. Status: proposed. Version 4.

Summary: #198: sc-lint uses the right Python itself

Scope: The item changes interpreter selection, runner config ([tool.sc-lint].runner), step argv or step environment in crates/sc-lint.

Deterministic check to pair with the development gate: Runner tests with stub interpreters show each atm-core step class gets its intended interpreter with no consumer per-OS path.

Fixtures:
- pass: sc-lint picks the repo venv interpreter for atm-core's python steps and clippy itself, so atm-core deletes _py, sc_lint_clippy_prefix and _sc-lint-clippy. Evidence: runner tests with stub interpreters pass, showing each step got the venv interpreter with no consumer per-OS path.
- fail: Leaves atm-core's _py recipe in place because steps still run whatever python is first on PATH.

## ANC-012

Anchor: sc-lint must be able to enforce dependency ownership on source files only.

> sc-lint enforces dependency ownership on source files only, and we know which of atm-core's three boundary settings it already covers.

Said by Rand, Phase M planning; Rand approved this sentence as his words ('yes'), relayed by team-lead 2026-10-30, 2026-10-30. Status: proposed. Version 5.

Summary: #136: dependency ownership on source files only

Scope: The item changes global_dependency_ownership or SCB-MANIFEST-004 in crates/sc-lint-boundary, or their config keys.

Deterministic check to pair with the development gate: A boundary test shows an ownership violation in src/ is reported and the same import in a tests/ target is not, under the source-only setting.

Fixtures:
- pass: Adds a source-only dependency ownership setting that checks src/ files and not tests/. Evidence: test ownership_source_only passes, showing a src/ violation is reported and the same import in tests/ is not.
- fail: Dependency ownership keeps checking every file, including tests/, and offers no way to limit it to source files.

## ANC-013

Anchor: Phase M must record which of atm-core's three boundary settings sc-lint already covers.

> sc-lint enforces dependency ownership on source files only, and we know which of atm-core's three boundary settings it already covers.

Said by Rand, Phase M planning; Rand approved this sentence as his words ('yes'), relayed by team-lead 2026-10-30, 2026-10-30. Status: retired. Version 3.

Summary: #136: record coverage of atm-core's three boundary settings

Scope: The item audits atm-core [boundaries] settings against sc-lint-boundary or writes a parity table.

Deterministic check to pair with the development gate: A parity table under docs/plans/phase-m/ has one covered/not-covered row per atm-core boundary setting, with evidence.

Fixtures:
- pass: Writes a table showing, for each of atm-core's three boundary settings, whether sc-lint covers it, with evidence.
- fail: Marks #136 done because some manifest rules shipped, without checking each of the three settings.

## GH-786.table

Anchor: PORT-010 must report none of the 47 findings listed for the 18 atm-core paths in sc-lint#786, because none is a true positive.

> Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design.
The 47 findings at this head match the issue's list exactly (same files, lines, item names)

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#786 comment 6513016641, 2026-10-30. Status: retired. Version 3.

Summary: #786 table: none of the 47 findings reported

Scope: The item changes PORT-010 classification in crates/sc-lint-portability or its #786 fixtures.

Deterministic check to pair with the development gate: Rerun the #786 per-path repro on atm-core 6ffaa97be with each path's exclusion removed: 0 of the 47 findings remain.

Fixtures:
- pass: The #786 per-path repro now reports none of the 47 listed findings.
- fail: Fixes the same-name sibling shape only, so the other #786 findings are still reported.

## GH-786.S3

Anchor: PORT-010 must not report a cfg(unix) item that has a same-name #[cfg(not(unix))] sibling in the same scope, whether that scope is a module, an inherent impl, or the crate root.

> S3  cfg(unix) item with a same-name `#[cfg(not(unix))]` sibling in the same scope (module, inherent impl, or crate root). Rows 2, 4, 6, 14, 18. This is #720 generalized beyond crate-root main.rs.

Said by atm-core (GitHub issue text), Issue acceptance text: https://github.com/randlee/sc-lint/issues/786#issuecomment-6513016641 (S3; generalizes #720), 2026-10-30. Status: proposed. Version 4.

Summary: #720/S3: same-name cfg(not(unix)) sibling, all scopes

Scope: The item changes PORT-010 sibling detection or port010 fixtures (crate_root_main, module, inherent impl).

Deterministic check to pair with the development gate: port010 fixtures for module, inherent-impl and crate-root same-name cfg(unix)/cfg(not(unix)) pairs report nothing.

Fixtures:
- pass: PORT-010 stops reporting a cfg(unix) item that has a same-name cfg(not(unix)) sibling in a module, an inherent impl or the crate root. Evidence: test port010_same_name_sibling passes, showing zero findings for fixtures of all three scopes.
- fail: Exempts the crate-root main.rs pair only; module and inherent-impl siblings are still reported.

## GH-198.setup-pre

Anchor: Consumer config must accept setup.pre, an argv that just setup runs before sc-lint provisioning.

> Proposal: `setup.pre` (argv run by `just setup` before sc-lint provisioning)

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#198 issue body, 2026-10-30. Status: retired. Version 3.

Summary: #198: [tool.sc-lint].setup.pre

Scope: The item edits ScLintConfigFile or consumer config parsing, or the just setup / installer flow.

Deterministic check to pair with the development gate: Config-parse test accepts setup.pre; a stub-command test shows it runs before provisioning.

Fixtures:
- pass: Adds setup.pre; just setup runs the configured argv before provisioning, shown by a stub-command ordering test.
- fail: Runs setup.pre after sc-lint provisioning has finished.

## GH-198.runner-argv

Anchor: When runner.python names a venv, a leading python in any lint or test step argv must resolve to that venv's interpreter on every OS.

> (1) a leading `"python"` in any lint **or test** step argv resolves to that venv's interpreter on every OS

Said by atm-core (GitHub issue text), Issue acceptance text: https://github.com/randlee/sc-lint/issues/198#issuecomment-6513016954 (a)(1),(3), 2026-10-30. Status: proposed. Version 4.

Summary: #198: runner.python resolves leading python, lint and test

Scope: The item changes step argv construction or runner.python handling in crates/sc-lint.

Deterministic check to pair with the development gate: Injected-platform argv tests: bin/python on Unix, Scripts\python.exe on Windows, for both a lint step and a test step.

Fixtures:
- pass: With runner.python set, a leading python in any lint or test step runs that venv's interpreter on every OS. Evidence: test runner_python_argv passes, showing .venv/bin/python on unix and .venv\Scripts\python.exe on windows for a lint step and a test step.
- fail: Applies runner.python to lint steps only; test steps still run python from PATH.

## GH-198.runner-path

Anchor: When runner.python names a venv, sc-lint must prepend that venv's bin directory (Scripts on Windows) to each step child's PATH.

> (2) the venv's bin/Scripts dir is prepended to the child's PATH (our `python_cmd` does this today so helpers that spawn `python` stay pinned)

Said by atm-core (GitHub issue text), Issue acceptance text: https://github.com/randlee/sc-lint/issues/198#issuecomment-6513016954 (a)(2), 2026-10-30. Status: retired. Version 3.

Summary: #198: runner.python venv dir prepended to child PATH

Scope: The item changes step child environment or runner.python handling in crates/sc-lint.

Deterministic check to pair with the development gate: A step-environment test with injected platform shows PATH starts with the venv bin or Scripts directory.

Fixtures:
- pass: Each step child gets PATH with the runner.python venv bin (Scripts on Windows) directory first.
- fail: Rewrites only the leading python argv, so a helper that spawns python picks up the PATH interpreter.

## GH-198.kit-helpers

Anchor: sc-lint must run sc_lint.* wheel helper steps on the kit interpreter and must never apply runner.python to them.

> plain `python` steps use `runner.python`; `sc_lint.*` wheel helpers always use the kit interpreter and `runner.python` must never apply to them.

Said by atm-core (GitHub issue text), Issue acceptance text: https://github.com/randlee/sc-lint/issues/198#issuecomment-6513016954 (b), 2026-10-30. Status: proposed. Version 4.

Summary: #198: sc_lint.* helpers on the kit interpreter

Scope: The item adds or changes how sc_lint.* helper steps (deny, manifests, shear, spell) are run in crates/sc-lint.

Deterministic check to pair with the development gate: A runner test with runner.python set shows an sc_lint.lint_cargo_deny step still resolves to the kit venv interpreter.

Fixtures:
- pass: sc_lint.* wheel helper steps always run on the kit interpreter, and runner.python is never applied to them. Evidence: test kit_helpers_ignore_runner_python passes, showing sc_lint.lint_cargo_deny resolves to the kit venv with runner.python set.
- fail: Applies runner.python to every step, including sc_lint.lint_cargo_deny.

## GH-198.export-env

Anchor: sc-lint must let a consumer export the runner.python interpreter as PYO3_PYTHON to steps that are not Python steps, such as cargo clippy.

> We need one more knob, e.g. `runner.export_env = ["PYO3_PYTHON"]` (plus the PATH prepend in (2) applied to every step), or a per-step `env`.

Said by atm-core (GitHub issue text), Issue acceptance text: https://github.com/randlee/sc-lint/issues/198#issuecomment-6513016954 (mismatch with ADR-021); NEEDS Rand scope ruling, 2026-10-30. Status: retired. Version 3.

Summary: #198: export PYO3_PYTHON for native steps (Rand scope ruling needed)

Scope: The item changes step environment, runner config, or clippy steps in crates/sc-lint.

Deterministic check to pair with the development gate: A step-environment test shows a cargo clippy step receives PYO3_PYTHON set to the runner.python interpreter when configured.

Fixtures:
- pass: Adds runner.export_env so a clippy step receives PYO3_PYTHON pointing at the runner.python interpreter.
- fail: Leaves PYO3_PYTHON unset for clippy, so atm-core keeps sc_lint_clippy_prefix.

## GH-136.src-only

Anchor: global_dependency_ownership must offer a config setting that limits the ownership check to src/**/*.rs and excludes integration-test targets.

> The previous atm-core checker passes on this tree because it scans `src/**/*.rs` and does not treat separate integration-test targets as production source. There is no config setting to preserve that source-only scope.

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#136 comment 2026-10-29, 2026-10-30. Status: retired. Version 3.

Summary: #136: source-only scope setting for global_dependency_ownership

Scope: The item changes global_dependency_ownership or SCB-MANIFEST-004 config in crates/sc-lint-boundary.

Deterministic check to pair with the development gate: Boundary test reproducing the #136 tree: the seven crates/*/tests/ findings disappear under the setting and crates/atm/src/commands/mod.rs is still reported.

Fixtures:
- pass: Adds a source-only setting; the #136 repro drops the seven tests/ findings and still reports crates/atm/src/commands/mod.rs.
- fail: Adds atm-core's tests/ directories to allowed_source_roots instead of adding a setting.

## GH-136.audit

Anchor: Phase M must check each of the three atm-core [boundaries] keys against released sc-lint-boundary and file a config-surface gap issue for each key not covered.

> Verify which of the three atm-core keys are covered and file the remainder as config-surface gaps.

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#136 issue body, 2026-10-30. Status: retired. Version 3.

Summary: #136: audit the three atm-core keys, file gaps

Scope: The item audits atm-core [boundaries] keys or files sc-lint-boundary config-surface gap issues.

Deterministic check to pair with the development gate: Parity table rows for manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules; each uncovered row links an issue.

Fixtures:
- pass: Checks all three atm-core keys and links a filed gap issue for each one sc-lint does not cover.
- fail: Checks the keys but records gaps only in a doc, with no gap issues filed.

## GH-786.table.2

Anchor: A release must stop PORT-010 reporting the 47 findings for the 18 atm-core paths in sc-lint#786, so atm-core can remove its PORT-010 exclusions.

> A release fixing the applicable findings allows atm-core to remove its PORT-010 exclusions.
[…]
Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design.
[…]
The 47 findings at this head match the issue's list exactly (same files, lines, item names)

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#786 body 'Requested outcome' + https://github.com/randlee/sc-lint/issues/786#issuecomment-6513016641, 2026-10-30. Status: retired. Version 3.

Summary: #786: none of the 47 findings reported, exclusions removable

Scope: The item changes PORT-010 classification in crates/sc-lint-portability or its #786 fixtures.

Deterministic check to pair with the development gate: Rerun the #786 per-path repro on atm-core 6ffaa97be with each path's exclusion removed: 0 of the 47 findings remain.

Fixtures:
- pass: Test port010_issue786_paths runs the 18 per-path repros against vendored atm-core 6ffaa97be sources and asserts 0 of the 47 findings remain; it passes in CI.
- fail: Fixes the same-name sibling shape only, so the other #786 findings are still reported.

## GH-198.setup-pre.2

Anchor: [tool.sc-lint].setup.pre must accept an argv that just setup runs before sc-lint provisioning.

> `setup.pre` (argv run by `just setup` before sc-lint provisioning)
[…]
[tool.sc-lint.setup]
pre = ["just", "bootstrap"]
[…]
**(c) Commitment:** yes, we commit to this split now and will adopt in the first release that ships it

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#198 body + https://github.com/randlee/sc-lint/issues/198#issuecomment-6513016954 (a),(c), 2026-10-30. Status: proposed. Version 3.

Summary: #198: [tool.sc-lint].setup.pre runs before provisioning

Scope: The item edits ScLintConfigFile or consumer config parsing, or the just setup / installer flow.

Deterministic check to pair with the development gate: Config-parse test accepts [tool.sc-lint].setup.pre; a stub-command test records it ran before provisioning.

Fixtures:
- pass: [tool.sc-lint].setup.pre takes an argv that just setup runs before sc-lint provisioning. Evidence: test setup_pre_runs_before_provisioning passes, showing the stub command ran before provisioning started.
- fail: Runs setup.pre after sc-lint provisioning has finished.

## GH-198.runner-path.2

Anchor: When runner.python names a venv, sc-lint must prepend that venv's bin directory (Scripts on Windows) to the PATH of every step's child process.

> (2) the venv's bin/Scripts dir is prepended to the child's PATH (our `python_cmd` does this today so helpers that spawn `python` stay pinned)
[…]
(plus the PATH prepend in (2) applied to every step)

Said by atm-core (GitHub issue text), Issue acceptance text: https://github.com/randlee/sc-lint/issues/198#issuecomment-6513016954 (a)(2) + mismatch paragraph, 2026-10-30. Status: proposed. Version 3.

Summary: #198: runner.python venv dir prepended to every step's PATH

Scope: The item changes step child environment or runner.python handling in crates/sc-lint.

Deterministic check to pair with the development gate: A step-environment test with injected platform shows PATH starts with the venv bin or Scripts directory for a python step and a native step.

Fixtures:
- pass: With runner.python set, sc-lint prepends the venv's bin (Scripts on Windows) directory to every step's PATH. Evidence: test runner_python_prepends_path passes, showing PATH starts with the venv directory for a python step and a cargo step.
- fail: Rewrites only the leading python argv, so a helper that spawns python picks up the PATH interpreter.

## GH-198.export-env.2

Anchor: sc-lint must be able to export the runner.python venv interpreter as PYO3_PYTHON to native (non-python) steps such as cargo clippy.

> `runner.python` only rewrites step argv; cargo clippy needs the venv interpreter exported as `PYO3_PYTHON` and the venv dir on PATH for native (non-python) steps.
[…]
We need one more knob, e.g. `runner.export_env = ["PYO3_PYTHON"]` (plus the PATH prepend in (2) applied to every step), or a per-step `env`.

Said by atm-core (GitHub issue text), Issue acceptance text: https://github.com/randlee/sc-lint/issues/198#issuecomment-6513016954 C + mismatch paragraph; NEEDS Rand scope ruling, 2026-10-30. Status: retired. Version 4.

Summary: #198: export PYO3_PYTHON to native steps (Rand scope ruling needed)

Scope: The item changes step environment, runner config, or clippy steps in crates/sc-lint.

Deterministic check to pair with the development gate: A step-environment test shows a cargo clippy step receives PYO3_PYTHON set to the runner.python interpreter when configured.

Fixtures:
- pass: sc-lint can export the runner.python interpreter as PYO3_PYTHON to native steps such as cargo clippy. Evidence: test runner_export_env_pyo3 passes, showing the clippy step's env has PYO3_PYTHON set to the venv interpreter.
- fail: Leaves PYO3_PYTHON unset for clippy, so atm-core keeps sc_lint_clippy_prefix.

## GH-136.src-only.2

Anchor: global_dependency_ownership must offer a source-only scope setting that checks src/**/*.rs and not integration-test targets, without consumers adding roots or excludes.

> The released 0.8.2 `global_dependency_ownership` policy has no source-only scope equivalent to the atm-core consumer check.
[…]
The previous atm-core checker passes on this tree because it scans `src/**/*.rs` and does not treat separate integration-test targets as production source. There is no config setting to preserve that source-only scope. atm-core therefore retains this rule locally pending a released source-only scope; no roots or excludes were added to suppress the findings.

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#136 comment 2026-10-29, 2026-10-30. Status: proposed. Version 3.

Summary: #136: source-only scope setting for global_dependency_ownership

Scope: The item changes global_dependency_ownership or SCB-MANIFEST-004 config in crates/sc-lint-boundary.

Deterministic check to pair with the development gate: Boundary test reproducing the #136 tree: the seven crates/*/tests/ findings disappear under the setting and crates/atm/src/commands/mod.rs is still reported.

Fixtures:
- pass: global_dependency_ownership gains a source-only scope setting that checks src/**/*.rs and not integration-test targets, with no roots or excludes added. Evidence: test issue136_source_only_scope passes, showing the seven crates/*/tests/ findings gone and crates/atm/src/commands/mod.rs still reported.
- fail: Adds atm-core's tests/ directories to allowed_source_roots instead of adding a setting.

## GH-136.audit.2

Anchor: For each of manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules, phase M must verify whether released sc-lint-boundary covers it and file a config-surface gap for each one not covered.

> atm-core `[boundaries]` config carries manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules (.just/lint-config.toml). Released sc-lint-boundary has manifest_policy.rs / package_policy.rs (--rule manifests / --rule dependencies, #73). Verify which of the three atm-core keys are covered and file the remainder as config-surface gaps.

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#136 issue body, 2026-10-30. Status: retired. Version 3.

Summary: #136: audit the three atm-core keys, file gaps

Scope: The item audits atm-core [boundaries] keys or files sc-lint-boundary config-surface gap issues.

Deterministic check to pair with the development gate: Parity table rows for manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules; each uncovered row links an issue.

Fixtures:
- pass: docs/plans/phase-m/boundary-parity.md has one row per key with the sc-lint-boundary test that covers it, and links issue #NNN for each uncovered key.
- fail: Checks the keys but records gaps only in a doc, with no gap issues filed.

## GH-136.audit-verify

Anchor: For each of manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules, phase M must verify whether released sc-lint-boundary covers it.

> atm-core `[boundaries]` config carries manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules (.just/lint-config.toml). Released sc-lint-boundary has manifest_policy.rs / package_policy.rs (--rule manifests / --rule dependencies, #73). Verify which of the three atm-core keys are covered and file the remainder as config-surface gaps.

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#136 issue body (split from GH-136.audit.2), 2026-10-30. Status: proposed. Version 3.

Summary: #136 audit: verify coverage of the three keys

Scope: The item audits atm-core [boundaries] keys against sc-lint-boundary or writes a parity table.

Deterministic check to pair with the development gate: Parity table under docs/plans/phase-m/ has a covered/not-covered row with evidence for each of the three keys.

Fixtures:
- pass: Verifies each of the three atm-core keys against released sc-lint-boundary and records covered or not covered. Evidence: docs/plans/phase-m/boundary-parity.md has one row per key citing the sc-lint-boundary test or code that covers it.
- fail: Verifies manifest_dependency_allowlists only and assumes the other two keys are covered.

## GH-136.audit-gaps

Anchor: Phase M must file a config-surface gap issue for each of the three atm-core [boundaries] keys that released sc-lint-boundary does not cover.

> atm-core `[boundaries]` config carries manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules (.just/lint-config.toml). Released sc-lint-boundary has manifest_policy.rs / package_policy.rs (--rule manifests / --rule dependencies, #73). Verify which of the three atm-core keys are covered and file the remainder as config-surface gaps.

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#136 issue body (split from GH-136.audit.2), 2026-10-30. Status: proposed. Version 3.

Summary: #136 audit: file a gap per uncovered key

Scope: The item files sc-lint-boundary config-surface gap issues or links them from the parity table.

Deterministic check to pair with the development gate: Each not-covered row in the parity table links a filed GitHub issue labelled as a config-surface gap.

Fixtures:
- pass: Files one config-surface gap issue for each atm-core key that sc-lint-boundary does not cover. Evidence: each not-covered row in docs/plans/phase-m/boundary-parity.md links its filed GitHub issue number.
- fail: Records uncovered keys in the parity table without filing any gap issues.

## ANC-014

Anchor: With atm-core's PORT-010 exclusions removed, PORT-010 must report no finding that sc-lint#786's table classifies as a false positive or deliberate exception.

> atm-core can remove its PORT-010 exclusions and get zero false alarms, while real Windows gaps are still reported.
Definition question (team-lead, 2026-10-30): "false alarm = a finding #786's table classifies as false positive or deliberate exception" Rand: "yes"

Said by Rand, Phase M planning; Rand approved the sentence and answered 'yes' to the definition question, relayed by team-lead 2026-10-30, 2026-10-30. Status: proposed. Version 2.

Summary: #786/#720: no PORT-010 false alarms on atm-core

Scope: The item changes PORT-010 in crates/sc-lint-portability, its fixtures or docs, or atm-core PORT-010 exclusions.

Deterministic check to pair with the development gate: #786 per-path repro on atm-core with every PORT-010 exclusion removed reports no finding the #786 table classifies as false positive or deliberate exception.

Fixtures:
- pass: PORT-010 stops reporting every #786 finding classified false positive or deliberate exception, so atm-core removes its exclusions. Evidence: test port010_issue786_paths passes, showing zero such findings with all exclusions removed.
- fail: PORT-010 still reports atm-core's same-name cfg(not(unix)) siblings, which #786 classifies as false positives.

## ANC-015

Anchor: PORT-010 must still report a cfg(unix) item that has no Windows, cfg(not(unix)) or portable counterpart.

> atm-core can remove its PORT-010 exclusions and get zero false alarms, while real Windows gaps are still reported.
Definition question (team-lead, 2026-10-30): "real Windows gap = a cfg(unix) item with no Windows, cfg(not(unix)) or portable counterpart" Rand: "yes"

Said by Rand, Phase M planning; Rand approved the sentence and answered 'yes' to the definition question, relayed by team-lead 2026-10-30, 2026-10-30. Status: proposed. Version 2.

Summary: #786/#720: real Windows gaps still reported

Scope: The item changes PORT-010 exemptions, classification or suppression in crates/sc-lint-portability.

Deterministic check to pair with the development gate: A port010 fixture with a cfg(unix) fn and no Windows, cfg(not(unix)) or portable counterpart still reports PORT-010.

Fixtures:
- pass: PORT-010 keeps reporting a cfg(unix) fn with no Windows, cfg(not(unix)) or portable counterpart. Evidence: test port010_real_gap_still_reported passes, showing one PORT-010 finding for that fixture.
- fail: Exempts every cfg(unix) fn in a file once any Windows code exists anywhere in that file.

## ANC-016

Anchor: The Phase M docs must contain a parity table recording whether sc-lint covers each of manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules.

> sc-lint enforces dependency ownership on source files only, and we know which of atm-core's three boundary settings it already covers.
Definition question (team-lead, 2026-10-30): "we know = written in a parity table in the Phase M docs for manifest_dependency_allowlists, global_dependency_ownership, manifest_section_rules" Rand: "yes"

Said by Rand, Phase M planning; Rand approved the sentence and answered 'yes' to the definition question, relayed by team-lead 2026-10-30, 2026-10-30. Status: proposed. Version 2.

Summary: #136: parity table of atm-core's three boundary settings

Scope: The item audits atm-core [boundaries] settings against sc-lint-boundary or writes a parity table in the Phase M docs.

Deterministic check to pair with the development gate: docs/plans/phase-m/ contains a parity table with one covered/not-covered row for each of the three settings.

Fixtures:
- pass: Writes a parity table in the Phase M docs with a covered or not-covered row for each of the three settings. Evidence: docs/plans/phase-m/boundary-parity.md has those three rows, each citing its source.
- fail: Records sc-lint coverage for manifest_dependency_allowlists only and leaves the other two settings out of the table.

## ANC-017

Anchor: PORT-010 must exempt pub cfg(unix)-only POSIX helpers that have no Windows counterpart because the concept does not exist, such as is_owned_by_uid and current_uid.

> S8  (only the pub caveat) pub cfg(unix)-only POSIX helpers (`is_owned_by_uid`, `current_uid` in atm_temp.rs) with no Windows counterpart because the concept does not exist; Windows uses a different check. Row 1. Sc-lint docs say pub items stay checked, so the sc-lint team must choose: exempt (S1 transitively) or keep reporting. Not a parity gap.
Rand ruling (relayed by team-lead 2026-10-30): "S8 = EXEMPT pub POSIX-only helpers"

Said by Rand, Phase M planning; #786 S8 text plus Rand's ruling relayed by team-lead 2026-10-30, 2026-10-30. Status: proposed. Version 2.

Summary: #786 S8: pub POSIX-only helpers exempt

Scope: The item changes PORT-010 handling of pub items or helper exemptions in crates/sc-lint-portability.

Deterministic check to pair with the development gate: A port010 fixture of atm_temp.rs-style pub is_owned_by_uid/current_uid helpers reports no PORT-010 finding; PORT-010 docs state the pub-helper exemption.

Fixtures:
- pass: PORT-010 stops reporting pub POSIX-only helpers like is_owned_by_uid and current_uid. Evidence: test port010_pub_posix_helper passes, showing zero findings for that fixture.
- fail: Keeps reporting pub is_owned_by_uid and current_uid because pub items stay checked.

## GH-786.table.3

Anchor: A release must stop PORT-010 reporting all 47 findings for the 18 atm-core paths in sc-lint#786, none of which is a true positive or parity gap, so atm-core can remove its PORT-010 exclusions.

> A release fixing the applicable findings allows atm-core to remove its PORT-010 exclusions.
[…]
Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design.
[…]
The 47 findings at this head match the issue's list exactly (same files, lines, item names)
[…]
S8  (only the pub caveat) pub cfg(unix)-only POSIX helpers (`is_owned_by_uid`, `current_uid` in atm_temp.rs) with no Windows counterpart because the concept does not exist; Windows uses a different check. Row 1. Sc-lint docs say pub items stay checked, so the sc-lint team must choose: exempt (S1 transitively) or keep reporting. Not a parity gap.

Said by atm-core (GitHub issue text), Issue acceptance text: sc-lint#786 body 'Requested outcome' + https://github.com/randlee/sc-lint/issues/786#issuecomment-6513016641 (incl. S8), 2026-10-30. Status: proposed. Version 2.

Summary: #786: none of the 47 findings reported, exclusions removable

Scope: The item changes PORT-010 classification in crates/sc-lint-portability or its #786 fixtures.

Deterministic check to pair with the development gate: Rerun the #786 per-path repro on atm-core 6ffaa97be with each path's exclusion removed: 0 of the 47 findings remain.

Fixtures:
- pass: PORT-010 stops reporting all 47 #786 findings, so atm-core removes its PORT-010 exclusions. Evidence: test port010_issue786_paths passes, showing 0 of the 47 findings across the 18 per-path repros.
- fail: Fixes the same-name sibling shape only, so the other #786 findings are still reported.

## GH-198.export-env.3

Anchor: sc-lint must be able to export the runner.python venv interpreter as PYO3_PYTHON to native (non-python) steps such as cargo clippy.

> `runner.python` only rewrites step argv; cargo clippy needs the venv interpreter exported as `PYO3_PYTHON` and the venv dir on PATH for native (non-python) steps.
[…]
We need one more knob, e.g. `runner.export_env = ["PYO3_PYTHON"]` (plus the PATH prepend in (2) applied to every step), or a per-step `env`.

Said by atm-core (GitHub issue text), Issue acceptance text: https://github.com/randlee/sc-lint/issues/198#issuecomment-6513016954 C + mismatch paragraph; in scope per Rand 2026-10-30, 2026-10-30. Status: proposed. Version 2.

Summary: #198: export PYO3_PYTHON to native steps

Scope: The item changes step environment, runner config, or clippy steps in crates/sc-lint.

Deterministic check to pair with the development gate: A step-environment test shows a cargo clippy step receives PYO3_PYTHON set to the runner.python interpreter when configured.

Fixtures:
- pass: sc-lint can export the runner.python interpreter as PYO3_PYTHON to native steps such as cargo clippy. Evidence: test runner_export_env_pyo3 passes, showing the clippy step's env has PYO3_PYTHON set to the venv interpreter.
- fail: Leaves PYO3_PYTHON unset for clippy, so atm-core keeps sc_lint_clippy_prefix.
