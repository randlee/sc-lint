# Final Quality Report

Generated: 2026-10-10T17:13Z
QA Pass: QA-2 (targeted)
Sprint/Task: stack-214-L6 lint QA-1 fixes / lint-8qz.10
Branch: `fix/stack214-lint-qa1`
Commit: `5e576ab`
PR: #217
Final Verdict: **PASS**

## Machine Status (JSON)
```json
{
  "sprint": "stack-214-L6 lint QA-1 fixes",
  "task": "lint-8qz.10",
  "branch": "fix/stack214-lint-qa1",
  "commit": "5e576ab",
  "pr": 217,
  "verdict": "PASS",
  "findings": {
    "blocking": 0,
    "important": 0,
    "minor": 2
  },
  "blocking_ids": "",
  "merge_readiness": "READY",
  "merge_reason": "QA-L1..L5 fixed or confirmed; CI 12/12 green on #217; only Minor findings remain",
  "next_action": "none",
  "action_owner": "none",
  "recommendation": "Merge with the stack; record QA-N1 and QA-N2 via /triaging-findings"
}
```

## Validated Scope
Deliverables 5/5 (100%). Reviewed exactly `f2f6e2f..5e576ab` (9 files, +318/-86; code +250/-86 plus the report doc and the cli-contract line). CI on #217: 12/12 pass. Python module tests: 30 pass. I built the layer's `sc-lint` and ran it end to end.

| ID | Disposition | Evidence |
|---|---|---|
| QA-L1 | FIXED | `python_adapter.rs` new `adapter_outcome` (:90-137) returns a `BackendFailure` when `exit_code != Some(0)` or `data.status == "fail"`. Three unit tests (`function_length_violation_fails_the_command`, `non_zero_exit_fails_even_when_the_payload_reports_pass`, `passing_verdict_keeps_the_success_envelope`). End to end on a scratch workspace with a 92-line new function: `sc-lint --json lint function-length` -> `ok:false`, `CLI.BACKEND_EXEC_FAILURE`, exit 5 (was exit 0). Passing runs unchanged: `lint function-length`, `lint line-counts`, `lint identity-literals` and `view findings` on this tree all exit 0 (`view_findings.py:54` reports top-level `status: pass`). |
| QA-L2 | FIXED | `renders_adapter_summary` removed (`command.rs` -11) and the explicit 4-variant arm restored at `render.rs:131-134`, so no new API rides in. Baseline records `render_success_human` 93 (`.just/function-length-baseline.tsv`); the 92 -> 93 deviation is the lead-accepted ruling. The gate passes against the tree. |
| QA-L3 | FIXED | Baseline identity is now `(path, name, occurrence)` (`lint_function_length.py:42-58,215-218`, 4-column TSV), so same-named functions no longer share an allowance (tests `test_same_named_functions_get_distinct_occurrences`, `test_new_same_named_function_cannot_hide_behind_an_existing_entry`). A shrunk function is reported `stale` (`:286`) and `--write-baseline` records its new length. `--write-baseline` refuses to raise or add an allowance unless `--allow-growth` is passed (`:252-266`; tests `test_write_refuses_*`). |
| QA-L4 | FIXED | `TEST_ATTRIBUTE_RE` (`:42`) accepts `test(...)` arguments; test `test_test_attributes_with_arguments_are_exempt_by_path` covers `#[tokio::test(flavor = "multi_thread")]`; `cfg(not(test))` and `expect(... "Tests" ...)` stay non-exempt. |
| QA-L5 | CONFIRMED (ruling) | `docs/sc-lint/cli-contract.md:153` states function-length is not in the `Ci` profile, gated by `just lint` and `Full`. |

### New findings (introduced by the fixes, both Minor)

- **QA-N1 (Minor)** The L1 failure envelope drops the violation list. `adapter_outcome` (`python_adapter.rs:105-111`) keeps only the summary plus `tool/script/status/exit_code`, so `sc-lint lint function-length` reports "1 new or grown function(s)" without naming it (observed in the end-to-end run); the names are only in the findings artifact `summary.txt`. Add the violations (or the artifact dir) to the error details.
- **QA-N2 (Minor)** Occurrence indexes (`lint_function_length.py:176-202`) are positional within a file, so inserting or deleting an earlier same-named function renumbers the rest. A baselined `fmt#1` becomes `fmt#2` and the gate reports a false "new" violation, and the only documented remedy is `--allow-growth`, which also accepts real growth. Also, a shrunk function is only reported stale (never failing), so its old allowance still permits regrowth until the baseline is regenerated.

Debt note (not a finding): `.gitignore` now ignores `**/.sc-compose/logs/`, which clears the untracked log noted in QA-1; it is outside L1..L5 but harmless.

## Findings Summary (Final)
- Blocking: 0
- Important: 0
- Minor: 2

## Residual Risks
Two Minor findings introduced by the fixes (QA-N1 violation list missing from the CLI failure envelope; QA-N2 positional occurrence index fragility).

## Merge Readiness
- Status: **READY**
- Reason: QA-L1..L5 fixed or confirmed; CI 12/12 green on #217; only Minor findings remain

## Recommendation
Merge with the stack; record QA-N1 and QA-N2 via /triaging-findings
