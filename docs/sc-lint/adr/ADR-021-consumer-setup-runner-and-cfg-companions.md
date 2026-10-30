# ADR-021 — Consumer Setup, Python Runner And Cfg Companions

| Field | Value |
| --- | --- |
| ID | ADR-021 |
| Status | Pending |
| Deciders | Rand |
| Related | ADR-010, ADR-016, ADR-020; #198, #720 |

## Context

The consumer evidence in #198 requires repository setup before product
provisioning and a repository Python environment for consumer steps. ADR-020
decision 9 currently declines both capabilities, and REQ-CLI-021 explicitly
excludes them. This proposal requires Rand's approval before implementation;
the accepted restrictions remain effective until then.

#720 records a second unresolved decision: PORT-010 reports a crate-root
`#[cfg(unix)]` function even when a same-name `#[cfg(not(unix))]` sibling
exists in that scope. The existing fixture pins that finding.

## Proposed Decision

1. **Consumer setup.** Support `[tool.sc-lint.setup].pre` as one non-empty
   argv array, for example `pre = ["python3", "tools/setup.py"]`. Bootstrap
   `setup` runs it once from the repository root, before binary discovery,
   downloads, wheel environment creation or declared-tool provisioning.
   Parse and validate configuration before running anything. Execute argv
   directly, without a shell. A nonzero child status stops setup and is
   returned unchanged; a spawn failure reports
   `CLI.SC_LINT_SETUP_PRE_FAILED`, command, cause and recovery, with exit 4.
   `--check` and `--dry-run` never execute the hook; dry-run displays the
   intended command. Upgrade, lint, test and exec do not execute it.
   The bootstrap uses only the standard library to read this input before
   the wheel exists. An absent hook preserves existing setup behavior.
2. **Repository Python.** Support `[tool.sc-lint.runner].python` as the
   non-empty path to a consumer virtual environment. Relative paths resolve
   from the repository root; absolute paths are accepted. Replace exactly a
   leading `"python"` in selected consumer lint and test step argv with
   `<venv>/bin/python` on Linux/macOS or `<venv>/Scripts/python.exe` on
   Windows. Preserve every subsequent argument, environment and scheduler
   dependency. Do not replace `python3`, executable paths or later tokens.
   With no setting, argv remains unchanged. Resolve only selected steps;
   missing interpreters fail before spawning with
   `CLI.SC_LINT_RUNNER_PYTHON_UNAVAILABLE`, path, cause, recovery and
   `sc-lint docs setup`, exit 4. `test --list` needs no interpreter.
   This selects a process executable at runtime; Rust retains its existing
   profile planning and scheduling, while bootstrap provisioning and asset
   generation remain Python/declarative under ADR-016. It adds no Rust
   scaffolding logic and no second Python profile planner.
3. **Structural companions.** A crate-root free function guarded by exactly
   `cfg(unix)` has PORT-010 parity when a non-test, same-name free function
   in the same item scope has exactly `cfg(not(unix))`. Retain explicit
   Windows companions and portable fallbacks. A differently named sibling,
   sibling in another scope, test-only sibling, `not(windows)`, or unrelated
   cfg does not establish this companion relationship. This decision does
   not generalize Boolean cfg equivalence or change impl-method parity.

On acceptance, decisions 1–2 supersede ADR-020 decision 9 and amend decision
13's closed key list to admit `setup` and `runner` under `tool.sc-lint`.
They supersede ADR-016's
restriction on setup hooks only for these two declared settings; amend
REQ-CLI-021 through proposed REQ-CLI-026. Decision 3 extends ADR-010's
structural parity contract for #720. All other accepted behavior remains.

## Configuration Errors

Malformed tables, unknown nested keys, an empty hook argv, non-string argv
elements, or an empty/non-string environment path fail as
`CLI.CONFIG_ERROR` (exit 3), with config path, field, cause, recovery and
`sc-lint docs configuration`. The accepted spelling is under
`tool.sc-lint`; root-level `setup` and `runner` remain unsupported.

## Approval And Scope

Rand's approval must cover both the #198 reversal and the #720
classification. The Phase M approval gate blocks m-1 and m-3 until this
record is Accepted and the CLI requirement amendment is activated on the
planning branch. Draft anchors are not approval.

This proposal excludes #198's kit-interpreter/module step form and #786's
18-path classification. It introduces no post hook, shell hook DSL, automatic
consumer environment creation or additional analyzer dependency.
