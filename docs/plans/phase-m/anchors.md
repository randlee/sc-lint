# Anchors - phase-m

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-m/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | proposed | A1 #786: PORT-010 per-shape classification of the 18 atm-core paths | 2026-10-30 |
| ANC-002 | proposed | A2 #720: PORT-010 crate-root cfg(unix)/cfg(not(unix)) companion | 2026-10-30 |
| ANC-003 | proposed | A3 #198: consumer setup.pre hook (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving).) | 2026-10-30 |
| ANC-004 | proposed | A4 #198: consumer runner.python (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving).) | 2026-10-30 |
| ANC-005 | proposed | A5 #198: wheel-only sc_lint helpers on the kit interpreter (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving). Design also awaits atm-core input.) | 2026-10-30 |
| ANC-006 | proposed | A6 #136: source-only scope for global_dependency_ownership | 2026-10-30 |
| ANC-007 | proposed | A7 #136: three-key parity audit | 2026-10-30 |

## ANC-001

Anchor: Each PORT-010 finding in sc-lint#786 must be resolved by shape classification: legitimate platform adapter or helper shapes stop being reported, and real cross-platform parity gaps are still reported.

> these are the p1 items from atm-core: - randlee/sc-lint#786 (PORT-010): all 18 excluded atm-core paths still emit 1–17 findings on 0.8.2, so I removed no exclusions. sc-lint.toml cites it with a release-based removal condition. This is the one that keeps atm-core exclusions in place.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; acceptance text from sc-lint#786 'Requested outcome' at atm-core d70ff224, 2026-10-30. Status: proposed. Version 2.

Summary: A1 #786: PORT-010 per-shape classification of the 18 atm-core paths

Scope: The item changes PORT-010 in crates/sc-lint-portability, adds or edits port010 fixtures, edits PORT-010 docs, or changes how atm-core PORT-010 exclusions or platform_adapters are handled.

Deterministic check to pair with the development gate: One regression fixture per classified shape under crates/sc-lint-portability/tests/fixtures/port010/ with both an exempt and a real-gap case; the #786 per-path repro (remove one exclude, run sc-lint --json lint sc-portability) on atm-core d70ff224 reports only shapes classified as real gaps.

Fixtures:
- pass: Adds one port010 fixture per classified #786 shape; the exempt adapter cases report nothing and the real-gap cases still report PORT-010.
- fail: Adds the 18 atm-core paths to a default PORT-010 exclusion list so the #786 repro reports nothing.

## ANC-002

Anchor: PORT-010 must give a crate-root #[cfg(unix)] fn with a #[cfg(not(unix))] sibling in the same file one documented classification, pinned by a fixture of atm-core's crates/atm/src/main.rs shape.

> these are the p1 items from atm-core: - randlee/sc-lint#720 (PORT-010): a crate-root cfg(unix) fn with a cfg(not(unix)) sibling is still reported. It covers only main.rs.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; acceptance criteria from sc-lint#720, 2026-10-30. Status: proposed. Version 2.

Summary: A2 #720: PORT-010 crate-root cfg(unix)/cfg(not(unix)) companion

Scope: The item changes PORT-010 sibling/companion detection, edits the port010 crate_root_main fixture or port010_fixture_crate_root_main_current_behaviour, or edits PORT-010 docs.

Deterministic check to pair with the development gate: port010_fixture_crate_root_main_current_behaviour (crates/sc-lint-portability/tests/fixtures/port010/crate_root_main) is replaced by a test pinning the decided classification, a real-gap variant still reports, and the PORT-010 docs diff states the classification.

Fixtures:
- pass: Replaces port010_fixture_crate_root_main_current_behaviour with a test pinning the decided classification and documents it in the PORT-010 docs.
- fail: Stops reporting the crate-root main.rs shape with no fixture change and no PORT-010 docs update.

## ANC-003

Anchor: The consumer sc-lint config must accept setup.pre, an argv list that just setup runs before sc-lint provisioning.

> these are the p1 items from atm-core: - randlee/sc-lint#198 (consumer setup hook and step runner): atm-core keeps _py, sc_lint_clippy_prefix and _sc-lint-clippy until it ships.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; proposal text from sc-lint#198. CONFLICT: ADR-020 decisions 9 and 13 (Accepted, Rand 2026-10-27) say setup.pre is not added and root-level setup is rejected with config.unknown_table, 2026-10-30. Status: proposed. Version 2.

Summary: A3 #198: consumer setup.pre hook (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving).)

Scope: The item edits ScLintConfigFile or consumer config parsing in crates/sc-lint (config.rs), the just setup / installer flow, config.unknown_table handling, or ADR-016/ADR-020 hook decisions.

Deterministic check to pair with the development gate: A config-parse test accepts setup.pre; a setup test with a stub command records that it ran before provisioning; ADR-020 decisions 9 and 13 are amended.

Fixtures:
- pass: Adds setup.pre to ScLintConfigFile; just setup runs the configured argv before provisioning, shown by a stub-command ordering test.
- fail: Keeps rejecting root-level setup with config.unknown_table, so consumers still need Justfile glue before just setup.

## ANC-004

Anchor: The consumer sc-lint config must accept runner.python, a venv path whose interpreter replaces a leading python in each lint and test step argv on Linux, macOS and Windows.

> these are the p1 items from atm-core: - randlee/sc-lint#198 (consumer setup hook and step runner): atm-core keeps _py, sc_lint_clippy_prefix and _sc-lint-clippy until it ships.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; proposal text from sc-lint#198 and atm-core comment 2026-10-29 item 2. CONFLICT: ADR-020 decisions 9 and 13 (Accepted, Rand 2026-10-27) say runner.python is not added, 2026-10-30. Status: proposed. Version 2.

Summary: A4 #198: consumer runner.python (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving).)

Scope: The item edits ScLintConfigFile or consumer config parsing, lint/test step argv construction in crates/sc-lint, or venv interpreter path resolution (bin/python vs Scripts\python.exe).

Deterministic check to pair with the development gate: Deterministic argv-rewrite unit tests over Unix and Windows venv path forms (bin/python, Scripts\python.exe) with injected platform, no real interpreter.

Fixtures:
- pass: Adds runner.python; a step argv starting with python runs the venv's bin/python on Unix and Scripts\python.exe on Windows, covered by injected-platform tests.
- fail: Runs a step's leading python from PATH regardless of the configured runner.python venv.

## ANC-005

Anchor: A consumer step must run a wheel-only sc_lint.<module> helper on the sc-lint kit's own interpreter without the consumer writing a per-OS interpreter path.

> these are the p1 items from atm-core: - randlee/sc-lint#198 (consumer setup hook and step runner): atm-core keeps _py, sc_lint_clippy_prefix and _sc-lint-clippy until it ships.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; need stated in atm-core comment on sc-lint#198 2026-10-29 item 1 (no step form to run a wheel-only module on the kit interpreter). Design undecided; awaits atm-core input, 2026-10-30. Status: proposed. Version 2.

Summary: A5 #198: wheel-only sc_lint helpers on the kit interpreter (CONDITIONAL on a new ADR superseding ADR-020 decisions 9/13 and REQ-CLI-021 (clint drafting, Rand approving). Design also awaits atm-core input.)

Scope: The item adds a step kind or subcommand for running sc_lint modules, edits consumer step execution in crates/sc-lint, or documents invoking .sc-lint/venv helpers.

Deterministic check to pair with the development gate: A runner test with a stub kit interpreter on Unix and Windows path forms shows a module step resolves to the kit venv interpreter; consumer config for it contains no OS-specific path.

Fixtures:
- pass: Adds a module step kind so {"module": "sc_lint.lint_cargo_deny"} runs on the kit venv interpreter on every OS.
- fail: Documents that consumers must call .sc-lint/venv/bin/python or Scripts\python.exe themselves for each OS.

## ANC-006

Anchor: global_dependency_ownership must offer a source-only scope in which SCB-MANIFEST-004 skips integration-test targets under crates/*/tests/ but still reports imports in production src files.

> these are the p1 items from atm-core: - randlee/sc-lint#136 (manifest dependency allowlists and global ownership versus the released manifest_policy): documented as a gap in the #2295 body.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; repro from atm-core comment on sc-lint#136 2026-10-29 (eight SCB-MANIFEST-004 findings on 0.8.2), 2026-10-30. Status: proposed. Version 2.

Summary: A6 #136: source-only scope for global_dependency_ownership

Scope: The item edits global_dependency_ownership or SCB-MANIFEST-004 in crates/sc-lint-boundary (manifest_dependency_policy), its config keys, or docs-bundle/packages/sc-lint-boundary.md.

Deterministic check to pair with the development gate: A boundary test reproducing the #136 tree: the seven crates/*/tests/ findings disappear under the source-only scope and crates/atm/src/commands/mod.rs importing cron is still reported; no allowed_source_roots or excludes added.

Fixtures:
- pass: Adds a source-only ownership scope; the #136 repro drops the seven crates/*/tests/ findings and still reports crates/atm/src/commands/mod.rs importing cron.
- fail: Tells atm-core to add its tests/ directories to allowed_source_roots or excludes to silence the findings.

## ANC-007

Anchor: Phase M must classify each atm-core [boundaries] key (manifest_dependency_allowlists, global_dependency_ownership, manifest_section_rules) as covered or not by released sc-lint-boundary, filing an issue per gap.

> these are the p1 items from atm-core: - randlee/sc-lint#136 (manifest dependency allowlists and global ownership versus the released manifest_policy): documented as a gap in the #2295 body.

Said by Rand, Phase M planning session; atm-core P1 list pasted by Rand; audit scope from sc-lint#136 body and team-lead A7 definition (2026-10-30), 2026-10-30. Status: proposed. Version 2.

Summary: A7 #136: three-key parity audit

Scope: The item audits or documents atm-core [boundaries] key parity, writes a parity table under docs/plans/phase-m/, or files sc-lint-boundary gap issues.

Deterministic check to pair with the development gate: A parity table under docs/plans/phase-m/ has one row per key with covered/gap and evidence; each gap row links a GitHub issue.

Fixtures:
- pass: Writes a parity table with one row per atm-core [boundaries] key, citing evidence, and links a filed GitHub issue for each gap row.
- fail: Declares #136 resolved because SCB-MANIFEST-003..006 shipped, without checking each key against atm-core behaviour.
