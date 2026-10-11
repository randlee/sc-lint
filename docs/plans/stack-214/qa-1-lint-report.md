# QA Findings Update

Generated: 2026-10-10T16:56Z
QA Pass: QA-1
Sprint/Task: stack-214-L4 function-length lint / lint-8qz.6
Branch: `feature/function-length-lint-v2`
Commit: `6bc4f4a`
PR: #215
Verdict: **FAIL**

## Machine Status (JSON)
```json
{
  "sprint": "stack-214-L4 function-length lint",
  "task": "lint-8qz.6",
  "branch": "feature/function-length-lint-v2",
  "commit": "6bc4f4a",
  "pr": 215,
  "verdict": "FAIL",
  "findings": {
    "blocking": 0,
    "important": 1,
    "minor": 4
  },
  "blocking_ids": [],
  "merge_readiness": "NOT READY",
  "merge_reason": "QA-L1: standalone `sc-lint lint function-length` exits 0 with ok:true on a violation",
  "next_action": "Fix QA-L1 (and QA-L2..L5 as triaged) on a new layer above #215; re-dispatch scoped QA-2",
  "owner": "team-lead"
}
```

## Findings Summary
- Blocking: 0
- Important: 1
- Minor: 4

## Blocking Findings
None blocking. Important: QA-L1.

## Detailed Findings
Reviewed exactly `origin/fix/qa-template-strict-xml..6bc4f4a` (18 files, +645/-13, matches the dispatch). CI on #215: 12/12 pass. Module unit tests (24) pass locally. No `Cargo.toml` change, no `sc-lint-attributes` dependency, no `fail_at` attribute added (the one `#[sc_lint(` hit is a test fixture string). The baseline reproduces from the tree with zero mismatches (13 entries, 13 over-cap, no stale entries).

### Verdict: FAIL (1 Important, 4 Minor)

**Checks that pass**
- Attribute parsing (old QA-I1): `lint_function_length.py:127-156` collects whole contiguous attributes (multi-line), exempts only exact `#[test]`, `#[path::test]`, `#[cfg(test)]` via `fullmatch` (`:43`), and resets `pending` on any non-attribute line, so there is no substring match and no inheritance from a previous item. Tests `test_lint_function_length.py:88-120` cover `cfg(not(test))`, `expect(... reason = "Tests ...")`, leakage across items and multi-line attributes.
- Literal text counts as code (old QA-I3): separate `code` and `structure` masks (`:100-120`).
- Baseline semantics: only new or grown over-cap functions fail (`evaluate`, `:262-272`); existing ones are recorded; shrunk ones are reported stale, not failing. I reproduced a new 92-line function: module exits 1.
- Growth `workflow.rs:lint_profile_plan` 97 -> 103: **acceptable as a baseline update.** The 6 lines are rustfmt's expansion of the required `python_step(...)` call (`workflow.rs:477-482`), and holding at <=97 would need an extraction refactor that this layer forbids. Keep the decision recorded in the sprint doc (done) and in the baseline diff.

**Findings**

- **QA-L1 (Important)** `crates/sc-lint/src/python_adapter.rs:78-118` (`run_python_tool`) ignores the child's exit status and `data.status`. Reproduced with the layer's built binary against a scratch workspace holding a new 92-line function: `sc-lint --json lint function-length` prints `"ok": true` with `data.status "fail"` and **exits 0**, while `python -m sc_lint.lint_function_length` exits 1. This is old QA-I2/QA-I9, still open. The hard gates (`just lint` via `run_lint.py:87-90`, and the Full profile step `workflow.rs:477`) call the module directly and do fail, so only the standalone CLI command is soft. Same pattern affects `line-counts`, `identity-literals` and `view findings` (all use `run_python_tool`). No Rust test covers a failing run: `tests.rs:1574-1584` only adds the command to the passing-envelope table. Fix: treat a non-zero exit or `status == "fail"` as a failure in `run_python_tool` and add a failing-case integration test.
- **QA-L2 (Minor)** Refactor of existing code despite the scope: `render.rs:131-139` rewrites the shared match arm to `command if command.renders_adapter_summary()` and adds a new `pub const fn renders_adapter_summary` (`command.rs:274-283`). `render_success_human` shrinks 92 -> 90 as a side effect, so the baseline records 90, not its pre-layer length.
- **QA-L3 (Minor)** Baseline identity is `(path, name)` (`lint_function_length.py:219-226`, `FunctionSpan.key`): same-named over-cap functions in one file (`new`, `fmt`, `from`) share one allowance, so a new function can hide behind an existing entry if it is not longer. Shrunk functions also keep their old allowance (no ratchet down): `from_cli` could regrow to 90 unnoticed after being cut. `--write-baseline` (`:236-243`) rewrites the file from the tree with no growth guard.
- **QA-L4 (Minor)** `TEST_ATTRIBUTE_RE` (`:43`) uses `fullmatch`, so `#[tokio::test(flavor = "multi_thread")]` (arguments) is not exempt outside a `cfg(test)` module. Consistent with the exact-forms rule but undocumented.
- **QA-L5 (Minor, old QA-M3)** The `Ci` profile (`workflow.rs:490-512`) omits `function-length` (as it omits `line-counts` and `identity-literals`), so `sc-lint lint ci` does not run the gate; only `Full` and `just lint` do. Decide and document.

Debt note: an untracked `bindings/sc-lint-py/python/.sc-compose/logs/sc-compose.log.jsonl` is present in the worktree (not part of the commit).

## Merge Readiness
- Status: **NOT READY**
- Reason: QA-L1: standalone `sc-lint lint function-length` exits 0 with ok:true on a violation

## Next Action
- Action: Fix QA-L1 (and QA-L2..L5 as triaged) on a new layer above #215; re-dispatch scoped QA-2
- Owner: team-lead
