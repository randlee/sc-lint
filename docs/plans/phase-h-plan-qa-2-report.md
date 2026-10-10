# QA Findings Update

Generated: 2026-10-10T15:47Z
QA Pass: QA-2 (targeted)
Sprint/Task: lint-3x2.5 (phase-h plan) / lint-3x2.5
Branch: `plans/phase-h`
Commit: `753e407`
PR: #206
Verdict: **FAIL**

## Machine Status (JSON)
```json
{
  "sprint": "lint-3x2.5 (phase-h plan)",
  "task": "lint-3x2.5",
  "branch": "plans/phase-h",
  "commit": "753e407",
  "pr": 206,
  "verdict": "FAIL",
  "findings": {
    "blocking": 0,
    "important": 2,
    "minor": 4
  },
  "blocking_ids": [],
  "merge_readiness": "NOT READY",
  "merge_reason": "2 Important and 4 Minor findings open; PASS needs 100% of carry-forward findings fixed",
  "next_action": "Fix QA2-001..006 on plans/phase-h (epic AC 1, lint-3x2.3 precondition, bead wording, stale ranges, RuleCatalog decision), then re-dispatch QA-3 scoped to these ids",
  "owner": "team-lead"
}
```

## Findings Summary
- Blocking: 0
- Important: 2
- Minor: 4

## Blocking Findings
None.

## Detailed Findings
### Dispositions (18 carry-forward findings): 12 FIXED, 6 NOT FIXED, 0 REGRESSED

| ID | Disposition | Evidence |
|---|---|---|
| PH-B1 | FIXED | Ruled narrowing: epic phase-h-epic:10-12, closure row :93, A-52 row :252; lint-h-3:358-359 consistent |
| PH-I1 | NOT FIXED | #186/lint-8qz.2 and merge-forward now in epic :64-74 and AC 6 :312-319; residual QA2-001 below |
| PH-I2 | NOT FIXED | ADR-015/016 line 9 relabelled; epic AC 1 (:296-301) still requires Accepted, see QA2-002 |
| PH-I3 | FIXED | Epic AC 6 :312-319 (1000-line budget, named sibling modules) |
| PH-I4 | FIXED | Epic AC 7 :320-327, shared helper in tests/support/mod.rs |
| PH-I5 | FIXED | A-53 :253; lint-h-12:626 no separate provision trait; marker owned once lint-h-3:635-638 |
| PH-I6 | FIXED | lint-h-4:75-81 bounds loosening to A-08/A-09, Y4 and G1 unchanged |
| PH-I7 | FIXED | Single RULE_FILTER_ERROR_MARKER lint-h-3:635-638, drift test AC15c :1017-1022 |
| PH-M1 | NOT FIXED | "approved in PR #206" remains: lint-h-1:21, h-2:23,31, h-3:28,39, h-4:22-23, h-5:28, h-7:24,26, h-12:28, h-13:25, h-14:23-24, h-11:1675 |
| PH-M2 | NOT FIXED | Epic ranges fixed (:136 T1-T23, :140 AC 1-39); sprint beads stale: lint-h-7:18,46,599 say T1-T22, lint-h-5:70 says AC 1-35 |
| PH-M3 | FIXED | Epic :98-100 integration-only ACs listed, AC 33 -> #197 |
| PH-M4 | FIXED | lint-h-13:54,642 "six rule sentences" |
| PH-M5 | FIXED | A-54 row :254 (per-issue GitHub comments not verifiable from exports) |
| PH-M6 | FIXED | lint-h-4:71-74 independent of h-3, no edge |
| PH-M7 | FIXED | docs/project-plan.md and docs/architecture.md point to phase h / 0.6.1 / lint-phase-h |
| PH-M8 | FIXED | Epic AC 7 :326-327,333-334 widened identity grep |
| PH-M9 | NOT FIXED | No sealed/open or static-dispatch decision for RuleCatalog (lint-h-3:444); selectors still raw String (:395,:428); no newtypes |
| PH-M10 | NOT FIXED | Known open: `validate-plan --phase h` needs integrate/phase-h (exit 2). `--phase h --index docs/plans/phase-h.jsonl` exits 0 (10 sprints). `--file` form still KeyError 'id' |

### New / residual findings

- **QA2-001 (Important, on PH-I1)**: epic :67-74 says lint-3x2.3 lands on develop before integrate/phase-h is cut; lint-3x2.3 was stopped 15:45Z (Rand), so the precondition is unmet. Reword or name another merge-forward point.
- **QA2-002 (Important, PH-I2)**: reword epic AC 1 to the ruled state (ADRs Proposed, Rand answers A-xx rows in PR #206; status gates adoption and merge only).
- **QA2-003 (Minor, PH-M1)**: reword "approved" to Pending in the listed beads and re-grep.
- **QA2-004 (Minor, PH-M2)**: update lint-h-7 T1-T22 -> T1-T23 and lint-h-5 AC 1-35 -> 1-39.
- **QA2-005 (Minor, PH-M9)**: record RuleCatalog sealed/static-dispatch decision and newtypes, or an explicit waiver.
- **QA2-006 (Minor, PH-M10, known open)**: live validate-plan blocked until integrate/phase-h exists.

CI at review time: no failures; 1 of 12 passed, the rest in progress. Stack: single layer, base develop, needsRebase false. Pending ADR status was not treated as a finding.

## Merge Readiness
- Status: **NOT READY**
- Reason: 2 Important and 4 Minor findings open; PASS needs 100% of carry-forward findings fixed

## Next Action
- Action: Fix QA2-001..006 on plans/phase-h (epic AC 1, lint-3x2.3 precondition, bead wording, stale ranges, RuleCatalog decision), then re-dispatch QA-3 scoped to these ids
- Owner: team-lead
