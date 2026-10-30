# Phase M fixture review

Prepared 2026-10-30 by flint for Rand's approval. Nothing is locked. Approving here approves each record's **pass** and **fail** sentence as the frozen fixtures the question tuner and the gates measure against; it does not lock the statements.

16 active records. Results are from the live runs committed under `anchor-runs/` (rubric r8, check-fixtures r8). Expected: pass → `satisfied`; fail → `not_addressed` or `contradicted`. ✗ marks a sentence that did not land as expected. A rubric cell is weak when it is not `yes` at confidence ≥ 0.6.

Reply per record: **approve**, **edit** (give the new sentence), or **drop**.

## Summary

| ID | Source | Rubric | Weak rubric cells | Pass (plan / dev) | Fail (plan / dev) |
|---|---|---|---|---|---|
| ANC-010 | Rand | inconclusive | single_constraint yes@0.4, faithful yes@0.55, self_contained yes@0.43 | satisfied / satisfied | contradicted / contradicted |
| ANC-011 | Rand | inconclusive | faithful yes@0.48, self_contained yes@0.55 | satisfied / satisfied | contradicted / contradicted |
| ANC-012 | Rand | inconclusive | faithful no@0.4 | satisfied / satisfied | underspecified ✗ / not_addressed |
| GH-786.S3 | issue text | testable | — | satisfied / satisfied | contradicted / contradicted |
| GH-198.runner-argv | issue text | inconclusive | faithful yes@0.55 | satisfied / satisfied | contradicted / contradicted |
| GH-198.kit-helpers | issue text | testable | — | satisfied / satisfied | contradicted / contradicted |
| GH-198.setup-pre.2 | issue text | inconclusive | self_contained yes@0.53 | satisfied / satisfied | contradicted / contradicted |
| GH-198.runner-path.2 | issue text | inconclusive | self_contained yes@0.56 | satisfied / satisfied | contradicted / contradicted |
| GH-136.src-only.2 | issue text | inconclusive | faithful None@None | satisfied / satisfied | contradicted / contradicted |
| GH-136.audit-verify | issue text | inconclusive | single_constraint yes@0.58 | satisfied / satisfied | contradicted / contradicted |
| GH-136.audit-gaps | issue text | inconclusive | faithful yes@0.56 | satisfied / satisfied | contradicted / contradicted |
| ANC-014 | Rand | inconclusive | faithful yes@0.42 | satisfied / satisfied | contradicted / contradicted |
| ANC-015 | Rand | testable | — | satisfied / satisfied | contradicted / contradicted |
| ANC-016 | Rand | testable | — | satisfied / satisfied | contradicted / contradicted |
| ANC-017 | Rand | inconclusive | faithful yes@0.43, self_contained yes@0.33 | satisfied / satisfied | contradicted / contradicted |
| GH-198.export-env.3 | issue text | inconclusive | single_constraint yes@0.55, faithful yes@0.43 | satisfied / satisfied | contradicted / contradicted |

## Open items

- **Questions for Rand** (the quote lacks the words; no rewording can supply them):
  - **ANC-010:** what is "the setup step"? Proposed: "the setup step is building atm-core's .bootstrap-venv (`just bootstrap`)".
  - **ANC-011:** what is "the right Python"? Proposed: "the right Python is the interpreter atm-core's _py, sc_lint_clippy_prefix and _sc-lint-clippy select today (.bootstrap-venv)".
  - **ANC-012:** "enforces dependency ownership on source files only": is source-only the **default**, or a **setting** a consumer turns on? GH-136.src-only.2 (issue text) asks for a setting.
  - **ANC-017:** does "exempt" apply to every pub POSIX-only helper, or only "S1 transitively" (when the helper's callers have a Windows counterpart), the option #786 S8 offered?
- **Noise-level cells:** every other weak cell is between 0.4 and 0.6 confidence, and unchanged statements move by ±0.1–0.2 between runs (GH-198.runner-path.2 testable in r6 and weak in r7/r8; GH-136.audit-verify testable in r7 and weak in r8). The edited records have had two statement rewordings each; the skill retires a record at three. Further rewording would chase noise.
- **ANC-012 fail** is `underspecified` at the plan gate: the default conflict question does not read "no way to limit it to source files" as contradicting the statement. Goes to the question tuner after approval; not reworded toward a pass.
- **GH-786.table.3** was dropped (retired 2026-10-30): ANC-014 and ANC-017 cover it.

## Records

### ANC-010 — #198: sc-lint runs the consumer setup step

**Statement:** sc-lint's setup command must itself run atm-core's setup step (today a Justfile recipe that builds .bootstrap-venv), so atm-core's Justfile no longer has to.

**Quote:**

> atm-core can delete its Justfile glue (_py, sc_lint_clippy_prefix, _sc-lint-clippy) because sc-lint runs the setup step and uses the right Python itself.

**Rubric:** inconclusive (weak: single_constraint yes@0.4, faithful yes@0.55, self_contained yes@0.43)

**Pass** (satisfied / satisfied): sc-lint setup runs the consumer's configured setup step itself, so atm-core deletes its Justfile setup recipe. Evidence: test setup_runs_configured_step passes, showing the configured stub command ran during sc-lint setup with no Justfile recipe.

**Fail** (contradicted / contradicted): Documents that consumers keep a Justfile recipe to run their own setup before sc-lint.

### ANC-011 — #198: sc-lint uses the right Python itself

**Statement:** sc-lint must itself select the Python interpreter for each atm-core lint and test step that atm-core's Justfile recipes _py, sc_lint_clippy_prefix and _sc-lint-clippy select today.

**Quote:**

> atm-core can delete its Justfile glue (_py, sc_lint_clippy_prefix, _sc-lint-clippy) because sc-lint runs the setup step and uses the right Python itself.

**Rubric:** inconclusive (weak: faithful yes@0.48, self_contained yes@0.55)

**Pass** (satisfied / satisfied): sc-lint picks the repo venv interpreter for atm-core's python steps and clippy itself, so atm-core deletes _py, sc_lint_clippy_prefix and _sc-lint-clippy. Evidence: runner tests with stub interpreters pass, showing each step got the venv interpreter with no consumer per-OS path.

**Fail** (contradicted / contradicted): Leaves atm-core's _py recipe in place because steps still run whatever python is first on PATH.

### ANC-012 — #136: dependency ownership on source files only

**Statement:** sc-lint's dependency ownership check (global_dependency_ownership) must be able to limit enforcement to source files only.

**Quote:**

> sc-lint enforces dependency ownership on source files only, and we know which of atm-core's three boundary settings it already covers.

**Rubric:** inconclusive (weak: faithful no@0.4)

**Pass** (satisfied / satisfied): Adds a source-only dependency ownership setting that checks src/ files and not tests/. Evidence: test ownership_source_only passes, showing a src/ violation is reported and the same import in tests/ is not.

**Fail** (underspecified ✗ / not_addressed): Dependency ownership keeps checking every file, including tests/, and offers no way to limit it to source files.

### GH-786.S3 — #720/S3: same-name cfg(not(unix)) sibling, all scopes

**Statement:** PORT-010 must not report a cfg(unix) item that has a same-name #[cfg(not(unix))] sibling in the same scope, whether that scope is a module, an inherent impl, or the crate root.

**Quote:**

> S3  cfg(unix) item with a same-name `#[cfg(not(unix))]` sibling in the same scope (module, inherent impl, or crate root). Rows 2, 4, 6, 14, 18. This is #720 generalized beyond crate-root main.rs.

**Rubric:** testable (weak: —)

**Pass** (satisfied / satisfied): PORT-010 stops reporting a cfg(unix) item that has a same-name cfg(not(unix)) sibling in a module, an inherent impl or the crate root. Evidence: test port010_same_name_sibling passes, showing zero findings for fixtures of all three scopes.

**Fail** (contradicted / contradicted): Exempts the crate-root main.rs pair only; module and inherent-impl siblings are still reported.

### GH-198.runner-argv — #198: runner.python resolves leading python, lint and test

**Statement:** When [tool.sc-lint].runner.python names a venv, a leading python in any lint or test step argv must resolve to that venv's interpreter on every OS.

**Quote:**

> (1) a leading `"python"` in any lint **or test** step argv resolves to that venv's interpreter on every OS

**Rubric:** inconclusive (weak: faithful yes@0.55)

**Pass** (satisfied / satisfied): With runner.python set, a leading python in any lint or test step runs that venv's interpreter on every OS. Evidence: test runner_python_argv passes, showing .venv/bin/python on unix and .venv\Scripts\python.exe on windows for a lint step and a test step.

**Fail** (contradicted / contradicted): Applies runner.python to lint steps only; test steps still run python from PATH.

### GH-198.kit-helpers — #198: sc_lint.* helpers on the kit interpreter

**Statement:** sc-lint must always run sc_lint.* wheel helper steps on the sc-lint kit's own venv interpreter (.sc-lint/venv), even when [tool.sc-lint].runner.python is set.

**Quote:**

> plain `python` steps use `runner.python`; `sc_lint.*` wheel helpers always use the kit interpreter and `runner.python` must never apply to them.

**Rubric:** testable (weak: —)

**Pass** (satisfied / satisfied): sc_lint.* wheel helper steps always run on the kit interpreter, and runner.python is never applied to them. Evidence: test kit_helpers_ignore_runner_python passes, showing sc_lint.lint_cargo_deny resolves to the kit venv with runner.python set.

**Fail** (contradicted / contradicted): Applies runner.python to every step, including sc_lint.lint_cargo_deny.

### GH-198.setup-pre.2 — #198: [tool.sc-lint].setup.pre runs before provisioning

**Statement:** [tool.sc-lint].setup.pre must accept an argv list that the sc-lint setup command (just setup) runs before it provisions sc-lint.

**Quote:**

> `setup.pre` (argv run by `just setup` before sc-lint provisioning)
> […]
> [tool.sc-lint.setup]
> pre = ["just", "bootstrap"]
> […]
> **(c) Commitment:** yes, we commit to this split now and will adopt in the first release that ships it

**Rubric:** inconclusive (weak: self_contained yes@0.53)

**Pass** (satisfied / satisfied): [tool.sc-lint].setup.pre takes an argv that just setup runs before sc-lint provisioning. Evidence: test setup_pre_runs_before_provisioning passes, showing the stub command ran before provisioning started.

**Fail** (contradicted / contradicted): Runs setup.pre after sc-lint provisioning has finished.

### GH-198.runner-path.2 — #198: runner.python venv dir prepended to every step's PATH

**Statement:** When runner.python names a venv, sc-lint must prepend that venv's bin directory (Scripts on Windows) to the PATH of every step's child process.

**Quote:**

> (2) the venv's bin/Scripts dir is prepended to the child's PATH (our `python_cmd` does this today so helpers that spawn `python` stay pinned)
> […]
> (plus the PATH prepend in (2) applied to every step)

**Rubric:** inconclusive (weak: self_contained yes@0.56)

**Pass** (satisfied / satisfied): With runner.python set, sc-lint prepends the venv's bin (Scripts on Windows) directory to every step's PATH. Evidence: test runner_python_prepends_path passes, showing PATH starts with the venv directory for a python step and a cargo step.

**Fail** (contradicted / contradicted): Rewrites only the leading python argv, so a helper that spawns python picks up the PATH interpreter.

### GH-136.src-only.2 — #136: source-only scope setting for global_dependency_ownership

**Statement:** global_dependency_ownership must offer a config setting that restricts its check to src/**/*.rs, the source-only scope atm-core's checker uses.

**Quote:**

> The released 0.8.2 `global_dependency_ownership` policy has no source-only scope equivalent to the atm-core consumer check.
> […]
> The previous atm-core checker passes on this tree because it scans `src/**/*.rs` and does not treat separate integration-test targets as production source. There is no config setting to preserve that source-only scope. atm-core therefore retains this rule locally pending a released source-only scope; no roots or excludes were added to suppress the findings.

**Rubric:** inconclusive (weak: faithful None@None)

**Pass** (satisfied / satisfied): global_dependency_ownership gains a source-only scope setting that checks src/**/*.rs and not integration-test targets, with no roots or excludes added. Evidence: test issue136_source_only_scope passes, showing the seven crates/*/tests/ findings gone and crates/atm/src/commands/mod.rs still reported.

**Fail** (contradicted / contradicted): Adds atm-core's tests/ directories to allowed_source_roots instead of adding a setting.

### GH-136.audit-verify — #136 audit: verify coverage of the three keys

**Statement:** Each of atm-core's three [boundaries] keys (manifest_dependency_allowlists, global_dependency_ownership, manifest_section_rules) must be classified as covered or not covered by the latest released sc-lint-boundary.

**Quote:**

> atm-core `[boundaries]` config carries manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules (.just/lint-config.toml). Released sc-lint-boundary has manifest_policy.rs / package_policy.rs (--rule manifests / --rule dependencies, #73). Verify which of the three atm-core keys are covered and file the remainder as config-surface gaps.

**Rubric:** inconclusive (weak: single_constraint yes@0.58)

**Pass** (satisfied / satisfied): Verifies each of the three atm-core keys against released sc-lint-boundary and records covered or not covered. Evidence: docs/plans/phase-m/boundary-parity.md has one row per key citing the sc-lint-boundary test or code that covers it.

**Fail** (contradicted / contradicted): Verifies manifest_dependency_allowlists only and assumes the other two keys are covered.

### GH-136.audit-gaps — #136 audit: file a gap per uncovered key

**Statement:** A config-surface gap issue must be filed in randlee/sc-lint for each atm-core [boundaries] key (manifest_dependency_allowlists, global_dependency_ownership, manifest_section_rules) that released sc-lint-boundary does not cover.

**Quote:**

> atm-core `[boundaries]` config carries manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules (.just/lint-config.toml). Released sc-lint-boundary has manifest_policy.rs / package_policy.rs (--rule manifests / --rule dependencies, #73). Verify which of the three atm-core keys are covered and file the remainder as config-surface gaps.

**Rubric:** inconclusive (weak: faithful yes@0.56)

**Pass** (satisfied / satisfied): Files one config-surface gap issue for each atm-core key that sc-lint-boundary does not cover. Evidence: each not-covered row in docs/plans/phase-m/boundary-parity.md links its filed GitHub issue number.

**Fail** (contradicted / contradicted): Records uncovered keys in the parity table without filing any gap issues.

### ANC-014 — #786/#720: no PORT-010 false alarms on atm-core

**Statement:** With atm-core's PORT-010 path exclusions removed, sc-lint's PORT-010 rule must report no finding on atm-core that the verdict table in sc-lint issue #786 classifies as a false positive or deliberate exception.

**Quote:**

> atm-core can remove its PORT-010 exclusions and get zero false alarms, while real Windows gaps are still reported.
> Definition question (team-lead, 2026-10-30): "false alarm = a finding #786's table classifies as false positive or deliberate exception" Rand: "yes"

**Rubric:** inconclusive (weak: faithful yes@0.42)

**Pass** (satisfied / satisfied): PORT-010 stops reporting every #786 finding classified false positive or deliberate exception, so atm-core removes its exclusions. Evidence: test port010_issue786_paths passes, showing zero such findings with all exclusions removed.

**Fail** (contradicted / contradicted): PORT-010 still reports atm-core's same-name cfg(not(unix)) siblings, which #786 classifies as false positives.

### ANC-015 — #786/#720: real Windows gaps still reported

**Statement:** sc-lint's PORT-010 rule must still report a #[cfg(unix)] item that has no #[cfg(windows)], #[cfg(not(unix))] or portable counterpart.

**Quote:**

> atm-core can remove its PORT-010 exclusions and get zero false alarms, while real Windows gaps are still reported.
> Definition question (team-lead, 2026-10-30): "real Windows gap = a cfg(unix) item with no Windows, cfg(not(unix)) or portable counterpart" Rand: "yes"

**Rubric:** testable (weak: —)

**Pass** (satisfied / satisfied): PORT-010 keeps reporting a cfg(unix) fn with no Windows, cfg(not(unix)) or portable counterpart. Evidence: test port010_real_gap_still_reported passes, showing one PORT-010 finding for that fixture.

**Fail** (contradicted / contradicted): Exempts every cfg(unix) fn in a file once any Windows code exists anywhere in that file.

### ANC-016 — #136: parity table of atm-core's three boundary settings

**Statement:** A parity table under docs/plans/phase-m/ must record whether sc-lint covers each of atm-core's boundary settings manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules.

**Quote:**

> sc-lint enforces dependency ownership on source files only, and we know which of atm-core's three boundary settings it already covers.
> Definition question (team-lead, 2026-10-30): "we know = written in a parity table in the Phase M docs for manifest_dependency_allowlists, global_dependency_ownership, manifest_section_rules" Rand: "yes"

**Rubric:** testable (weak: —)

**Pass** (satisfied / satisfied): Writes a parity table in the Phase M docs with a covered or not-covered row for each of the three settings. Evidence: docs/plans/phase-m/boundary-parity.md has those three rows, each citing its source.

**Fail** (contradicted / contradicted): Records sc-lint coverage for manifest_dependency_allowlists only and leaves the other two settings out of the table.

### ANC-017 — #786 S8: pub POSIX-only helpers exempt

**Statement:** sc-lint's PORT-010 rule must not report a pub #[cfg(unix)] function that has no Windows counterpart because Windows uses a different check, such as is_owned_by_uid and current_uid in crates/atm-core/src/atm_temp.rs.

**Quote:**

> S8  (only the pub caveat) pub cfg(unix)-only POSIX helpers (`is_owned_by_uid`, `current_uid` in atm_temp.rs) with no Windows counterpart because the concept does not exist; Windows uses a different check. Row 1. Sc-lint docs say pub items stay checked, so the sc-lint team must choose: exempt (S1 transitively) or keep reporting. Not a parity gap.
> Rand ruling (relayed by team-lead 2026-10-30): "S8 = EXEMPT pub POSIX-only helpers"

**Rubric:** inconclusive (weak: faithful yes@0.43, self_contained yes@0.33)

**Pass** (satisfied / satisfied): PORT-010 stops reporting pub POSIX-only helpers like is_owned_by_uid and current_uid. Evidence: test port010_pub_posix_helper passes, showing zero findings for that fixture.

**Fail** (contradicted / contradicted): Keeps reporting pub is_owned_by_uid and current_uid because pub items stay checked.

### GH-198.export-env.3 — #198: export PYO3_PYTHON to native steps

**Statement:** sc-lint must be able to export the [tool.sc-lint].runner.python venv interpreter as PYO3_PYTHON to native (non-python) steps such as cargo clippy.

**Quote:**

> `runner.python` only rewrites step argv; cargo clippy needs the venv interpreter exported as `PYO3_PYTHON` and the venv dir on PATH for native (non-python) steps.
> […]
> We need one more knob, e.g. `runner.export_env = ["PYO3_PYTHON"]` (plus the PATH prepend in (2) applied to every step), or a per-step `env`.

**Rubric:** inconclusive (weak: single_constraint yes@0.55, faithful yes@0.43)

**Pass** (satisfied / satisfied): sc-lint can export the runner.python interpreter as PYO3_PYTHON to native steps such as cargo clippy. Evidence: test runner_export_env_pyo3 passes, showing the clippy step's env has PYO3_PYTHON set to the venv interpreter.

**Fail** (contradicted / contradicted): Leaves PYO3_PYTHON unset for clippy, so atm-core keeps sc_lint_clippy_prefix.

