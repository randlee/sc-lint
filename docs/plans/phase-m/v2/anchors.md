# Anchors - phase-m

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `../intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | MS1 | 2026-10-30 |
| ANC-002 | locked | MS2 | 2026-10-30 |
| ANC-003 | locked | MS3 | 2026-10-30 |
| ANC-004 | locked | MS4 | 2026-10-30 |
| ANC-005 | locked | MS5 | 2026-10-30 |
| ANC-006 | locked | MS6a | 2026-10-30 |
| ANC-007 | locked | MS6b | 2026-10-30 |
| ANC-008 | locked | MS7 | 2026-10-30 |
| ANC-009 | locked | C04 | 2026-10-30 |
| ANC-010 | locked | C08 | 2026-10-30 |
| ANC-011 | locked | C09 | 2026-10-30 |
| ANC-012 | locked | C10 | 2026-10-30 |
| ANC-013 | locked | C11 | 2026-10-30 |
| ANC-014 | locked | C12 | 2026-10-30 |

## ANC-001

Anchor: sc-lint's PORT-010 rule (which reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) must not report a private or pub(crate) #[cfg(unix)] helper whose only callers are #[cfg(unix)] items that themselves have a #[cfg(windows)] or #[cfg(not(unix))] sibling.

> Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S1  Private (or pub(crate)) cfg(unix) helper whose only callers are cfg(unix) items that themselves have a Windows/not(unix) sibling. Rows 1, 2, 3, 5, 6, 7, 10, 11.

Said by ISSUE #786 (atm-core per-path verdict table, 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate MS1, 2026-10-30. Status: locked. Version 2.

Summary: MS1

Scope: The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 stops reporting a private #[cfg(unix)] helper whose only callers are #[cfg(unix)] functions that have #[cfg(windows)] siblings. Evidence: test port010_s1_helper_of_paired_fn passes, showing zero findings for that fixture.
- fail: PORT-010 still reports a private #[cfg(unix)] helper even though its only caller has a #[cfg(windows)] sibling.

## ANC-002

Anchor: sc-lint's PORT-010 rule (which reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) must not report a #[cfg(unix)] function whose #[cfg(windows)] companion exists under a different name, such as connect_unix paired with connect_named_pipe.

> Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S2  cfg(unix) fn whose cfg(windows) companion has a different name (`connect_unix` / `connect_named_pipe`, `unix_default_scratch_root` / `windows_default_scratch_root`, `unix_synthesized_..._path` / `windows_synthesized_..._path`). Rows 1, 5, 9.

Said by ISSUE #786 (atm-core per-path verdict table, 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate MS2, 2026-10-30. Status: locked. Version 2.

Summary: MS2

Scope: The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 accepts a differently named #[cfg(windows)] companion such as connect_named_pipe for connect_unix. Evidence: test port010_s2_renamed_companion passes, showing zero findings for that pair.
- fail: PORT-010 still reports connect_unix although connect_named_pipe is its #[cfg(windows)] companion.

## ANC-003

Anchor: sc-lint's PORT-010 rule (which reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) must not report a #[cfg(unix)] item that has a same-name #[cfg(not(unix))] sibling in the same scope, whether that scope is a module, an inherent impl or the crate root.

> ## Distinct shapes (what a rule must recognize)

- S3  cfg(unix) item with a same-name `#[cfg(not(unix))]` sibling in the same scope (module, inherent impl, or crate root). Rows 2, 4, 6, 14, 18. This is #720 generalized beyond crate-root main.rs.

Said by ISSUE #786 (atm-core per-path verdict table, 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate MS3, 2026-10-30. Status: locked. Version 2.

Summary: MS3

Scope: The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 stops reporting a #[cfg(unix)] item with a same-name #[cfg(not(unix))] sibling in a module, an inherent impl and the crate root. Evidence: test port010_s3_same_name_sibling passes, showing zero findings for all three scopes.
- fail: PORT-010 exempts the crate-root main.rs pair only; module and inherent-impl same-name siblings are still reported.

## ANC-004

Anchor: sc-lint's PORT-010 rule must recognize, and not report, a #[cfg(unix)] transport or adapter function or match arm that sits beside a loopback TCP enum variant or function with no #[cfg] guard, which Windows uses instead.

> Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S4  cfg(unix) transport/adapter fn or match arm beside an unguarded loopback-TCP sibling (enum variant or fn) that serves Windows. Rows 8, 11, 12, 15.

Said by ISSUE #786 (atm-core per-path verdict table, 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate MS4, 2026-10-30. Status: locked. Version 3.

Summary: MS4

Scope: The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 stops reporting a #[cfg(unix)] transport match arm that sits beside an un-gated loopback TCP enum variant used on Windows. Evidence: test port010_s4_loopback_tcp_sibling passes, showing zero findings for that fixture.
- fail: PORT-010 still reports the #[cfg(unix)] Unix-socket match arm even though the TcpLoopback enum variant beside it has no #[cfg] guard and serves Windows.

## ANC-005

Anchor: sc-lint's PORT-010 rule (which reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) must not report a #[cfg(unix)] helper used only inside a #[cfg(unix)] block of a function whose #[cfg(not(unix))] block is in the same function body.

> Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S5  cfg(unix) helper used only inside a cfg(unix) block of a function whose cfg(not(unix)) block is in the same body. Rows 10, 16.

Said by ISSUE #786 (atm-core per-path verdict table, 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate MS5, 2026-10-30. Status: locked. Version 2.

Summary: MS5

Scope: The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 stops reporting a #[cfg(unix)] helper used only inside a #[cfg(unix)] block of a function whose #[cfg(not(unix))] block is in the same body. Evidence: test port010_s5_cfg_blocks_in_one_fn passes, showing zero findings for that fixture.
- fail: PORT-010 still reports the helper called from the #[cfg(unix)] block although the same function has a #[cfg(not(unix))] block.

## ANC-006

Anchor: sc-lint's PORT-010 rule (which reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) must not report the declaration #[cfg(unix)] mod x; of a module that is never compiled on Windows.

> Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S6  Module declared `#[cfg(unix)] mod x;` and items inside it also carry cfg(unix); module is never compiled on Windows. Rows 13 (the decl), 17.

Said by ISSUE #786 (atm-core per-path verdict table, 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate MS6a, 2026-10-30. Status: locked. Version 2.

Summary: MS6a

Scope: The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 no longer reports a #[cfg(unix)] mod unix_socket; declaration. Evidence: test port010_s6_unix_module_decl passes, showing zero findings for the declaration line.
- fail: PORT-010 still reports the #[cfg(unix)] mod unix_socket; declaration line.

## ANC-007

Anchor: sc-lint's PORT-010 rule must recognize, and not report, items carrying #[cfg(unix)] inside a module declared #[cfg(unix)] mod x;, since that module is never compiled on Windows.

> Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S6  Module declared `#[cfg(unix)] mod x;` and items inside it also carry cfg(unix); module is never compiled on Windows. Rows 13 (the decl), 17.

Said by ISSUE #786 (atm-core per-path verdict table, 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate MS6b, 2026-10-30. Status: locked. Version 2.

Summary: MS6b

Scope: The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 no longer reports #[cfg(unix)] items inside a module declared #[cfg(unix)] mod unix_socket;. Evidence: test port010_s6_items_in_unix_module passes, showing zero findings inside that module.
- fail: PORT-010 still reports each #[cfg(unix)] function inside the #[cfg(unix)] mod unix_socket module.

## ANC-008

Anchor: sc-lint's PORT-010 rule (which reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) must not report a #[cfg(unix)] accessor or consumer of a cross-platform config type when non-Unix code deliberately rejects that config with a typed error.

> Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S7  cfg(unix) accessor or consumer of a cross-platform config type where non-unix deliberately rejects the config with a typed error. Rows 13 (`get`), 15.

Said by ISSUE #786 (atm-core per-path verdict table, 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate MS7, 2026-10-30. Status: locked. Version 2.

Summary: MS7

Scope: The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 stops reporting a #[cfg(unix)] accessor on a cross-platform config type that non-Unix validation rejects with a typed error. Evidence: test port010_s7_rejected_config_accessor passes, showing zero findings for that fixture.
- fail: PORT-010 still reports the #[cfg(unix)] get() accessor although validate_config rejects that config on non-Unix with a typed error.

## ANC-009

Anchor: sc-lint's PORT-010 (sc-lint's rule that reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) changes must not blanket-suppress real parity gaps: a production #[cfg(unix)] item with no #[cfg(windows)] companion and no portable fallback must still be reported.

> #720 covers only the crate-root main.rs shape; this issue tracks the complete remaining inventory and asks for per-case classification, not blanket suppression of real parity gaps.

Said by ISSUE #786 (Scope), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate C04, 2026-10-30. Status: locked. Version 2.

Summary: C04

Scope: The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item.

Fixtures:
- pass: PORT-010 keeps reporting a production #[cfg(unix)] function with no #[cfg(windows)] companion and no portable fallback. Evidence: test port010_real_gap_still_reported passes, showing one finding for that fixture.
- fail: PORT-010 now skips every #[cfg(unix)] item in a file once any Windows code exists anywhere in that file.

## ANC-010

Anchor: When [tool.sc-lint].runner.python names a repository venv directory, a leading "python" in the argv of any sc-lint lint or test step must run that venv's interpreter (bin/python, or Scripts\python.exe on Windows) on Linux, macOS and Windows.

> Semantics we need from `runner.python`: (1) a leading `"python"` in any lint **or test** step argv resolves to that venv's interpreter on every OS; (2) the venv's bin/Scripts dir is prepended to the child's PATH (our `python_cmd` does this today so helpers that spawn `python` stay pinned); (3) it must apply to `test` steps, not only `lint` ones.

Said by ISSUE #198 (atm-core comment 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate C08, 2026-10-30. Status: locked. Version 2.

Summary: C08

Scope: The item changes how sc-lint resolves the interpreter for lint or test step argv.

Fixtures:
- pass: With runner.python set to .venv, a leading python in a lint step and in a test step runs .venv/bin/python on Unix and .venv\Scripts\python.exe on Windows. Evidence: test runner_python_argv passes for a lint step and a test step on Linux, macOS and Windows.
- fail: runner.python rewrites a leading python only in lint steps; test steps still run whatever python is on PATH.

## ANC-011

Anchor: When [tool.sc-lint].runner.python names a repository venv directory, sc-lint must prepend that venv's bin directory (Scripts on Windows) to the PATH of every lint and test step's child process.

> Semantics we need from `runner.python`: (1) a leading `"python"` in any lint **or test** step argv resolves to that venv's interpreter on every OS; (2) the venv's bin/Scripts dir is prepended to the child's PATH (our `python_cmd` does this today so helpers that spawn `python` stay pinned); (3) it must apply to `test` steps, not only `lint` ones.

Said by ISSUE #198 (atm-core comment 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate C09, 2026-10-30. Status: locked. Version 2.

Summary: C09

Scope: The item changes the environment sc-lint passes to lint or test step child processes.

Fixtures:
- pass: With runner.python set, sc-lint prepends the venv's bin directory (Scripts on Windows) to every step's PATH. Evidence: test runner_python_prepends_path passes, showing PATH starts with the venv directory for a lint step and a test step.
- fail: runner.python rewrites only the leading python argv, so a helper that spawns python picks up the interpreter on PATH.

## ANC-012

Anchor: sc-lint must run cargo clippy steps (clippy, and clippy-windows via xwin) with the PYO3_PYTHON and PATH environment variables pointing at the Python interpreter inside the repository's .bootstrap-venv virtual environment directory.

> C. `clippy` and `clippy-windows` (xwin) need cargo to see `PYO3_PYTHON` and `PATH` pointing at the `.bootstrap-venv` interpreter (`sc_lint_clippy_prefix` + `_sc-lint-clippy` in the Justfile).

Said by ISSUE #198 (atm-core comment 2026-10-30), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate C10, 2026-10-30. Status: locked. Version 2.

Summary: C10

Scope: The item changes the environment sc-lint passes to cargo clippy steps.

Fixtures:
- pass: sc-lint runs clippy and clippy-windows with PYO3_PYTHON and PATH pointing at the .bootstrap-venv interpreter. Evidence: test clippy_step_env passes, showing both variables point into .bootstrap-venv for both steps.
- fail: clippy runs with PYO3_PYTHON unset, so atm-core keeps its sc_lint_clippy_prefix recipe.

## ANC-013

Anchor: sc-lint setup must run the argv configured at [tool.sc-lint].setup.pre before it provisions the sc-lint kit.

> `setup.pre` (argv run by `just setup` before sc-lint provisioning)

Said by ISSUE #198 (body), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate C11, 2026-10-30. Status: locked. Version 2.

Summary: C11

Scope: The item changes the consumer setup or bootstrap command.

Fixtures:
- pass: sc-lint setup runs the [tool.sc-lint].setup.pre argv before it provisions the kit. Evidence: test setup_pre_runs_before_provisioning passes, showing the stub command ran before provisioning started.
- fail: sc-lint setup runs setup.pre only after provisioning has finished.

## ANC-014

Anchor: sc-lint must provide a step form, such as a module step kind or a documented sc-lint lint subcommand, that runs a named sc_lint.* wheel helper module on the sc-lint kit's interpreter on every OS without a consumer-written per-OS interpreter path.

> The wheel-only helpers (`sc_lint.lint_cargo_deny`, `lint_cargo_shear`, `lint_manifests`, `lint_codespell`) have to be invoked as `.sc-lint/venv/bin/python -m ...` on every OS (`Scripts\python.exe` on Windows). There is no step form that says "run this module on the kit's interpreter", so we carry a 3-line `_sc-lint-py` recipe with a per-OS interpreter path. A step kind like `{"module": "sc_lint.lint_cargo_deny"}` or a documented `sc-lint lint deny|shear|manifests|spell` subcommand would delete it.

Said by ISSUE #198 (atm-core comment 2026-10-29), Phase M v2 re-derivation from REQ/ADR and issue text, 2026-10-30; candidate C12, 2026-10-30. Status: locked. Version 2.

Summary: C12

Scope: The item changes how a consumer step invokes an sc_lint.* wheel helper module.

Fixtures:
- pass: Adds a module step kind that runs sc_lint.lint_cargo_deny on the sc-lint kit interpreter on every OS. Evidence: test module_step_uses_kit_interpreter passes, showing the step used the kit venv interpreter on Linux, macOS and Windows with no per-OS path in the config.
- fail: Consumers must still write .sc-lint/venv/bin/python and Scripts\python.exe per OS to run sc_lint.* helpers.
