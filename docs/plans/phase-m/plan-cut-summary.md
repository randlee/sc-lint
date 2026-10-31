# Phase M four-sprint cut (lint-m-plancut)

Rand, 2026-10-31: "proceed w/ : Cut to the 4-sprint set and apply the ceremony removals".

Applied the brief and atm-core sign-off addendum. The live sprint beads are authoritative. The four-sprint plan is [phase-m-plan.md](phase-m-plan.md).

## Planning gate evidence

| Sprint | Run ID | Result |
| --- | --- | --- |
| lint-m-1 | [plan-20261031T191655Z-lint-m-1](v3/anchor-runs/plan-gate-cut-20261031/lint-m-1.run.json) | PASS |
| lint-m-2 | [plan-20261031T191655Z-lint-m-2](v3/anchor-runs/plan-gate-cut-20261031/lint-m-2.run.json) | PASS |
| lint-m-3 | [plan-20261031T191655Z-lint-m-3](v3/anchor-runs/plan-gate-cut-20261031/lint-m-3.run.json) | FAIL: ANC-012 |
| lint-m-4 | [plan-20261031T191655Z-lint-m-4](v3/anchor-runs/plan-gate-cut-20261031/lint-m-4.run.json) | PASS |
| lint-m-3 | [plan-20261031T191803Z-lint-m-3](v3/anchor-runs/plan-gate-cut-20261031-adr-cites/lint-m-3.run.json) | PASS |

The first m-3 run had ANC-012 relevance confidence 0.67, then 0.58 on confirmation; addressed=yes and contradicted=no were both 1.0 on both asks. The script retained an unstable/underspecified verdict. After the lead identified a real stale ADR reference, only ADR-022 decision 3.2 citations were corrected to decision 3. The lead authorized one m-3 rerun, which passed. Both results are retained and posted on the bead. No bead wording was changed for ANC-012, and no question was tuned.

The latest four runs pass. Coverage below is computed by `jev_judge.py coverage` across both run directories, selecting the latest run per sprint. All eight kept anchors are covered.

## Anchor coverage (plan)

| Anchor | lint-m-1 | lint-m-2 | lint-m-3 | lint-m-4 | Covered |
| --- | --- | --- | --- | --- | --- |
| ANC-003 | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-006 | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-007 | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-009 | satisfied | not applicable | not applicable | satisfied | yes |
| ANC-010 | not applicable | not applicable | not applicable | satisfied | yes |
| ANC-011 | not applicable | not applicable | satisfied | not applicable | yes |
| ANC-012 | not applicable | not applicable | satisfied | not applicable | yes |
| ANC-013 | not applicable | satisfied | not applicable | not applicable | yes |



## Applied changes

- m-1 owns S3/S6 and updates existing fixture exact lists. The inventory, extraction/hash and burndown work is removed.
- m-4 owns all remaining #786 shapes through reasoned site allowances, including the sign-off attribute forms, no-op attribute contract and consumer dependency/boundary adoption docs.
- m-2 includes #385 cycle-path filtering; m-3 includes #195 output and steps/ logs. The specified redundant tests and replay are removed.
- Five superseded sprint containers and their existing groups are closed. m-7/m-8 had no poured group beads. m-6 and its groups were already closed.
- Sprint QA acceptance requires matching the issue Planned delivery comment and posting approval on every targeted issue.
- Plan QA no longer blocks dispatch; ADR-010 gates m-1, ADR-022 gates m-4 only. There are no owner-based waits.
- The root no longer depends on the overdue-issue epic. Per the lead ruling, standalone lint-phase-m-done depends on the five closure beads; direct root closure edges would cycle through sprint parent-child links. #720 closure is merged into #786; #195/#385 closure cites sprint evidence without rerunning tests.
- Five anchors were retired with the approved reason; the kept records carry sprint mappings. Statements, scopes, quotes, fixtures and sealed questions are unchanged.
- Historical evidence uses the literal variable names ans and NOTH. Their approved codespell entries preserve that evidence without editing it.

## Validation

- Anchor record validation: PASS, no warnings.
- `validate-plan --ci`: PASS, four tracked phase files.
- Dependency cycles: none.
- Live `validate-plan --phase m --index docs/plans/phase-m.jsonl`: four reported limitations: root metadata lacks integration_branch; the old stack ledger does not match integrate/phase-m; root integration_branch required by its schema; unrelated top-level lint-wsgn has no model. These were reported to the lead. The approved four stacks target develop; no unapproved integration hierarchy or unrelated-bead repair was added.
- Codespell: PASS before final commit.
- `just lint`: PASS.
- `just test`: PASS.

PR #788 is updated by one authorized push after final validation. Hosted CI is recorded in the task handoff.
