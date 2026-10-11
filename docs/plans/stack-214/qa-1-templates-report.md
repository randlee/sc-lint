# Final Quality Report

Generated: 2026-10-10T16:46Z
QA Pass: QA-1
Sprint/Task: stack-214 (#211<-#212<-#213) / lint-8qz.4
Branch: `fix/qa-template-strict-xml`
Commit: `5e9d22b`
PR: #213
Final Verdict: **PASS**

## Machine Status (JSON)
```json
{
  "sprint": "stack-214 (#211\u003c-#212\u003c-#213)",
  "task": "lint-8qz.4",
  "branch": "fix/qa-template-strict-xml",
  "commit": "5e9d22b",
  "pr": 213,
  "verdict": "PASS",
  "findings": {
    "blocking": 0,
    "important": 0,
    "minor": 2
  },
  "blocking_ids": "",
  "merge_readiness": "READY",
  "merge_reason": "All three layers correct and still needed; CI 12/12 green on #211, #212, #213; no blocking findings",
  "next_action": "none",
  "action_owner": "none",
  "recommendation": "Merge stack #214; record QA-N1 and QA-M6 via /triaging-findings"
}
```

## Validated Scope
Reviewed exactly origin/develop(2b32e55)..5e9d22b: 3 commits, 11 files, +257/-81, matching the dispatched sums (L1 +32/-14, L2 +75/-28, L3 +150/-39). Nothing else rides in the range: only `.claude/**/*.j2` and `test_orchestration_contracts.py`. CI: #211, #212, #213 each 12/12 pass.

| Layer | PR | Verdict | Evidence |
|---|---|---|---|
| L1 findings as JSON arrays | #211 (8c33fe0) | PASS | `carry_forward_findings_json \| safe` in all 8 assignment templates (e.g. `.claude/skills/codex-orchestration/ruthless-boundary-qa-assignment.json.j2:36`, `arch-qa-assignment.json.j2:48`, `rust-qa-assignment.json.j2:53`); develop still lacks `\| safe`, so it is still needed; template versions bumped; test `test_carry_forward_findings_render_as_json_arrays`. |
| L2 machine-status escaping | #212 (f1f001a) | PASS | Every machine-status field in `findings-report.md.j2` and `quality-report.md.j2` now `\| tojson`, so quotes/backslashes/newlines stay valid JSON and counts stay numbers; test `test_quality_report_machine_json_round_trips_strings`. I have used these templates in this session and they render valid JSON. |
| L3 strict XML | #213 (5e9d22b) | PASS | `{% autoescape false %}` removed; every element body is `<![CDATA[... \| string \| cdata_escape]]>`. I rendered `qa-template.xml.j2` with sc-compose 1.6.1 using values containing `"`, `<`, `&`, `]]>` and a ragged `lead`/`cc`: the output parsed as well-formed XML, attributes were entity-escaped, `]]>` round-tripped intact. Test `test_qa_template_renders_strict_xml_with_typed_values`. |

### Findings

- **QA-N1 (Minor, L3, test)**: `test_orchestration_contracts.py:377-393` runs `sc-compose render --file <abs template>` with no `cwd` and no `--root`. sc-compose treats cwd as the root, so run from `bindings/sc-lint-py/python` it fails with `ERR_RESOLVE_NOT_FOUND: template path escapes configured roots` (rc 3, reproduced here). It passes only when cwd contains `.claude/` (CI does). Fix: pass `cwd=REPO` or `--root REPO`.
- **QA-M6 applicability (Minor, pre-existing, not introduced by #211)**: the scope lock is a string compare `carry_forward_findings_json != "[]"` at `ruthless-boundary-qa-assignment.json.j2:37-38` and `rust-best-practices-assignment.json.j2:45-46`. #211 does not touch those lines, but it makes the array the supported input, so a caller passing an empty array as `[ ]` or `[\n]` silently loses the lock and gets `findings_scope_locked: false`. Applies to the same two files; fix by comparing a parsed length or normalising input, on a later layer.

### Debt notes
`review_targets`, `commits` etc. are covered by the test only as lists; real dispatches pass multi-line strings (rendered correctly in my manual check). The test run here could not execute fully on this host (pytest and jinja2 absent); I relied on CI plus my own sc-compose render.

## Findings Summary (Final)
- Blocking: 0
- Important: 0
- Minor: 2

## Residual Risks
Two Minor findings (QA-N1 test cwd dependence; QA-M6 string-compare scope lock) to be fixed on a new layer.

## Merge Readiness
- Status: **READY**
- Reason: All three layers correct and still needed; CI 12/12 green on #211, #212, #213; no blocking findings

## Recommendation
Merge stack #214; record QA-N1 and QA-M6 via /triaging-findings
