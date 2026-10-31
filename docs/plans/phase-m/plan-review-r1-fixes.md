# Phase M plan review round 1 fixes

Task lint-phase-m-plan-fix1 applies the lead's accepted rulings from
2026-10-31. The live sprint beads remain authoritative. No implementation
or new tests are claimed here: the changes specify the future sprint work.

## Accepted findings applied

| Area | Applied plan change |
| --- | --- |
| m-1 metadata | Added REQ-PRODUCT-004A, ADR-017 and ADR-022; retained REQ-PRODUCT-004AA/ADR-010. Owned docs/requirements.md and a deliverable reconcile its S1-S7 wording. |
| m-2 metadata | Added REQ-PRODUCT-023 and ADR-020 to REQ-SCB-019; ADR-022 is context only. |
| m-3 metadata | Added REQ-CLI-023/027, REQ-LOG-002/003/004 and ADR-017/020; retained ADR-016 and REQ-CLI-021. ADR-022 is context only. |
| m-4 metadata | Added REQ-PRODUCT-003/004D/002DA, REQ-SCB-001/007 and ADR-004/007/011/013/020 to the prior portability authority. |
| m-1 S6 scope | Named lint-m-adr010 as the gate for Rand's public-scope ruling; acceptance tests whichever rule is approved, plus the unallowed/Windows-compiled negative. The implementer does not choose the ruling. |
| m-1 fixtures | Explicit ownership of atm_core fixture roots/README; regression tests record exact develop lists before the fix, then post-S3/S6 lists. |
| m-1 shared resolution | Neutral private module_cfg.rs is the sole module-resolution/propagation implementation used by adapters and parity; existing adapter and new parity tests protect both callers. |
| m-1/m-4 overlap | Both designs name portability/parity.rs, lib.rs and tests.rs; m-4 owns tests.rs. m-4 lands first; lead merges develop forward into the second branch. No must_follow edge or duplicate m-1 merge sentence. |
| m-2 ADR amendment | Added ADR-020 Amendment 2026-10-31, Pending Rand, extending decision 5 to SCB-CYCLE-001. The bead and plan reference it. |
| m-2 fallback tests | Named manifest004_macro_body_textual, manifest004_parse_failure_fails_closed and manifest004_doc_and_string_not_uses; one named textual fallback function, restricted callers and explicit deletion condition. |
| m-3 log paths | One step_log_file_name function removes separators/dot components, uses unique expanded-plan ordinals, and overwrites each executed step's file per run. One private step_logs.rs owns output I/O. Named safety/collision and overwrite tests. |
| m-3 types/errors | Exact shared parser signatures and env/cwd types; InvalidStepEnv/InvalidStepCwd unit variants, stable detail codes, field/step context and recovery. profile_plan.rs owns profile errors; scheduler.rs remains scheduling. |
| m-3 docs | Named docs-bundle/configuration.md clippy example; owned scheduler.rs, logging/docs files and step_logs.rs. Added acceptance for docs and unchanged behavior. |
| m-3 new ID | Under the New Ids exception, metadata lists REQ-CLI-027, docs/sc-lint/cli-requirements.md is owned, and deliverable 6 explicitly adds that requirement with the step-output contract. Its definition lands during m-3; historical REQ-CLI-026 is not reused. |
| m-4 report schema | Own v1 and new v2 schema artifacts, validator and fixtures. Declare optional allowed on v2; preserve strict v1 and unchanged 0.1.0 no-allowance reports. Reports with allowances use 0.2.0/v2, because old v1 validators reject unknown properties. The lead accepted this compatibility choice. |
| m-4 errors/types | Stable DirectiveError/PortabilityError codes, causes, recovery and docs references; validated private SiteRuleId and reason invariants. New error/entry types use the stated non_exhaustive decisions; Rust API additions are assessed under ADR-011. |
| m-4 grammar | Header and every requested edge case are explicit, including empty/trailing/duplicate/malformed/non-literal forms, unknown PORT error and other-analyzer ignore. Qualified AllowedEntry.item format is fixed. |
| m-4 boundary/docs | Own docs/architecture.md, portability docs and directives traceability; cite ADR-022 decision 4/ADR-013. portability forbidden_edges remains unchanged. |
| Ownership | clint owns m-1 then m-2; cfast owns m-4 then m-3. Group owner metadata and any set assignees follow those owners. Plan capacity is two, without new dependency edges. |
| Repeated prose | Removed the repeated no-blanket-suppression phrase from both Does Not Close sections. |

## Rejected findings recorded for round 2

- No expanded Branch Stacks And Parallelism section, repeated stack protocol
  or additional ADR status ceremony: the plan keeps its CLAUDE.md pointer,
  per the approved ceremony removal.
- m-4 remains one layer; the suggested split was not taken.
- No new global cross-analyzer rule-prefix ownership gate/test. The requested
  grammar's other-analyzer ignore case remains in the existing corpus work.

## Initial planning gate results

All four changed beads were judged once with unchanged sealed questions.
Run IDs are plan-20261031T194639Z-lint-m-1 through lint-m-4. Reports are
retained under v3/anchor-runs/plan-gate-review-fix1-20261031 and posted on
the judged beads. No post-gate bead edit was made to obtain a pass.

| Cell | Result and evidence |
| --- | --- |
| m-1 / ANC-006 | underspecified; addressed yes 1.0, contradicted no 0.55. The S6 declaration outcome is explicit; the new acceptance also names the pending public-scope ruling. |
| m-1 / ANC-010 | underspecified; relevance yes 0.34, addressed no 0.58. m-1 mentions m-4's allowance for a public item if Rand restricts S6 scope; m-4 owns the allowance surface. |
| m-3 / ANC-012 | unstable/underspecified; relevance yes 0.61 then 0.49; addressed yes and contradicted no both 1.0 on both asks. The setup/runner prohibition remains explicit. |
| m-4 / ANC-009 | underspecified; addressed yes 0.60, contradicted no 0.34. The bead keeps unannotated unrelated siblings reporting while implementing the approved site opt-out. |

m-2 passes. The other three fail as recorded; there is no planning-gate
PASS claim. The lead sent these cells to independent anchor-tuning triage
and instructed no push until that ruling. Statements, scopes, fixtures and
questions were unchanged in that initial repair iteration.

## Anchor coverage (plan)

| Anchor | lint-m-1 | lint-m-2 | lint-m-3 | lint-m-4 | Covered |
| --- | --- | --- | --- | --- | --- |
| ANC-003 | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-006 | underspecified | not applicable | not applicable | not applicable | no |
| ANC-007 | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-009 | satisfied | not applicable | not applicable | underspecified | yes |
| ANC-010 | underspecified | not applicable | not applicable | satisfied | yes |
| ANC-011 | not applicable | not applicable | satisfied | not applicable | yes |
| ANC-012 | not applicable | not applicable | underspecified | not applicable | no |
| ANC-013 | not applicable | satisfied | not applicable | not applicable | yes |

Uncovered anchors (no item satisfies them): ANC-006, ANC-012


## Validation

- Live validate-plan: PASS, four sprints.
- Offline validate-plan --ci: PASS, four tracked phase files.
- Initial just lint: failed only because codespell flagged the prior completion wording
  in an untracked prior completion text. That local prose was corrected. A second attempt found the same word in
  the first captured lint log because scratchpad logs are scanned too. Both
  diagnostics were preserved outside the checkout; no archived questions or
  gate evidence were altered.
- Final just lint: PASS. Final just test: PASS.
- Initial push hold: resolved by the follow-up ruling below.

## Approved triage follow-up

Lead ruling 01M69RNHZNCNMMETK3MYN0G5R6 separates one plan defect from three
independently proven question defects. m-1 now states the exact ANC-006
rule: the cfg(unix) mod declaration is never reported, regardless of the
item-scope ruling. Only items inside the module remain subject to Rand's
public-scope decision (ANC-007).

The independent frozen-oracle results for ANC-009/010/012 were accepted by
the lead. The planner verified their oracle hashes, sealed question contents
and provenance, and copied overrides-plan.proposed.json byte-for-byte.
Only those three question sets changed; no anchor statement, scope, fixture
or production threshold changed. Full triage evidence and provenance are
retained in v3/anchor-runs/tuning/fix1-triage-20261031/.

Fixture checks: 16 PASS, zero mismatches. The three affected sprint gates
PASS; m-2 retains its prior pass. Latest run IDs:

- plan-20261031T195949Z-lint-m-1
- plan-20261031T195949Z-lint-m-3
- plan-20261031T195949Z-lint-m-4
- plan-20261031T194639Z-lint-m-2 (unchanged sprint).

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



All four latest sprint gates pass; all eight kept anchors are covered.
Live and offline validate-plan, codespell, final just lint and final just
test all PASS. The single push is authorized by the lead's conditional
triage ruling now that every latest sprint passes and coverage is complete.
