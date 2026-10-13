# Phase K anchor review

All nine sprint descriptions pass at threshold 0.6 using the accepted question sets. All nine locked anchors are satisfied by at least one sprint. The source audit confirms that each run used the complete current bead description; no item was truncated. Quality-mgr round 3 returned FAIL with one important finding. Its diagnostic-string duplication is corrected, but that correction has not received independent QA verification; this is anchor approval only.

Rand’s intent: “This plan should use code and TESTS directly from atm-core repo where it makes sense”; “We should implement tools in rust as appropriate w/ minimal python bindings”; and “sc-lint is a general purpose linting tool that is used on many repos.”

## Current results

| Sprint | Result | Run |
| --- | --- | --- |
| lint-k-1 | PASS | [plan-20261013T034016Z-lint-k-1](anchor-runs/qa-r3/plan-20261013T034016Z-lint-k-1.json) |
| lint-k-2 | PASS | [plan-20261013T033844Z-lint-k-2](anchor-runs/qa-r3/plan-20261013T033844Z-lint-k-2.json) |
| lint-k-3 | PASS | [plan-20261013T033851Z-lint-k-3](anchor-runs/qa-r3/plan-20261013T033851Z-lint-k-3.json) |
| lint-k-4 | PASS | [plan-20261013T033836Z-lint-k-4](anchor-runs/qa-r3/plan-20261013T033836Z-lint-k-4.json) |
| lint-k-6 | PASS | [plan-20261013T033905Z-lint-k-6](anchor-runs/qa-r3/plan-20261013T033905Z-lint-k-6.json) |
| lint-k-7 | PASS | [plan-20261013T033935Z-lint-k-7](anchor-runs/qa-r3/plan-20261013T033935Z-lint-k-7.json) |
| lint-k-8 | PASS | [plan-20261013T034542Z-lint-k-8](anchor-runs/qa-r3/plan-20261013T034542Z-lint-k-8.json) |
| lint-k-9 | PASS | [plan-20261013T033852Z-lint-k-9](anchor-runs/qa-r3/plan-20261013T033852Z-lint-k-9.json) |
| lint-k-10 | PASS | [plan-20261013T033858Z-lint-k-10](anchor-runs/qa-r3/plan-20261013T033858Z-lint-k-10.json) |

## Evidence and changes

- [Coverage](anchor-coverage.md) is computed by the gate script from exactly the nine selected runs.
- [Source audit](anchor-runs/final/source-audit.json) binds complete live descriptions to current anchors and question digests.
- [Accepted questions](anchor-questions.md) include unchanged development questions and independently tuned planning questions.
- ANC-004 tuning distinguishes changed callable bindings from installer glue and tests of unchanged bindings. It passed all 17 frozen controls. ANC-007 tuning clarifies that existing-config bootstrap operations affect consumer compatibility; it passed all 16 frozen controls. Approved intent, fixtures, and the 0.6 threshold are unchanged.
- Plan clarifications preserve optional-policy absence behavior, simpler-consumer operation, upstream code/test reuse, Rust/Python responsibility, and concrete issue-derived regressions. Added passages trace to previously locked ANC-001 through ANC-008 and accepted ADR-017. The final documentation sprint remains in scope under ANC-009.
- Raw failed runs remain historical evidence, not approval. Earlier generic calibration did not pass; no generic calibration pass is claimed.
- [Review evidence archive](anchor-runs/final/review-evidence.tar.gz) retains complete source snapshots, change dispositions, frozen controls, raw tuning results, and prior QA findings. Run differences are under `anchor-runs/final/diffs/`.

## Adoption status

Plan schema validation passes for nine sprints. ADR-017 is Accepted. QA round 3 cleared seven of its eight carried findings. The remaining root/sprint diagnostic-string mismatch now has one source in k8 D3 and an exact-output assertion. The QA bead remains open; a further verification requires an exception to the three-round cap. No implementation has been dispatched. `just lint` and `just test` both passed after the final correction.

Historical raw runs and tuning proof are retained in `anchor-runs/history.tar.gz`; current run JSON remains directly readable.
