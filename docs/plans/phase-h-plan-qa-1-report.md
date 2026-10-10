# QA Findings Update

Generated: 2026-10-10 15:38 UTC
QA Pass: Plan-QA (docs-only plan review)
Sprint/Task: phase-h / lint-3x2.4
Branch: `plans/phase-h`
Commit: `6d5eb90fc80ec1f977aec90ee8472fe6b6225583`
PR: #206
Verdict: **FAIL**

## Machine Status (JSON)
```json
{
  "sprint": "phase-h",
  "task": "lint-3x2.4",
  "branch": "plans/phase-h",
  "commit": "6d5eb90fc80ec1f977aec90ee8472fe6b6225583",
  "pr": "206",
  "verdict": "FAIL",
  "findings": {
    "blocking": 1,
    "important": 7,
    "minor": 10
  },
  "blocking_ids": "PH-B1",
  "merge_readiness": "NOT READY",
  "merge_reason": "1 blocking, 7 important, 10 minor. Deliverable completion 12/20 (60%) per req-qa. Coverage: all of #193-#204 map to sprints with ACs; #201 is only partly covered (PH-B1). Ordering (be-1/2/3/4, #201 before #202/#203, #200 early) and the jsonl/beads dependency wiring verified. Pending ADRs 005/012/015/016 not treated as a finding; only approval wording is reported. CI on #206 passes; branch is 1 merge (#207 plan-anchoring skill) behind develop, so use the three-dot diff and merge develop forward before adoption. Review limits: parts of beads h-2, h-3, h-5, h-11, h-12 were checked by AC heading only; service-hardening and schema-reviewer reviewers not run (not in standing sc-lint set / no such agent); origin/integrate/phase-h does not exist so plans/phase-h was substituted.",
  "next_action": "Fix all findings in plan content (beads and #206) on a new layer, triage via /triaging-findings, then request QA-2.",
  "owner": "team-lead"
}
```

## Findings Summary
- Blocking: 1
- Important: 7
- Minor: 10

## Blocking Findings
- **PH-B1** `bead lint-h-3 (COMMON_ANALYZER_KEYS, AC1-AC50); docs/requirements.md REQ-PRODUCT-023; GitHub #201` — #201 asks for common keys `enabled`, `rules` (include by id/family), `exclude_rules`, `fail_rules`, per-rule `severity` overrides, `paths`/`exclude_paths`. The plan's `COMMON_ANALYZER_KEYS` is `[enabled, reason, exclude_rules, fail_rules, exclude_paths]` (lint-h-3 line 357-358): `rules`, `severity` and `paths` are neither planned nor recorded as a ruled narrowing; `severity` appears nowhere in the plan docs or epic decisions. This breaks the epic Goal 'all twelve issues, nothing deferred' and leaves #201 with no way to close. (req-qa ATM-QA-001; verified) _Fix:_ Either add `rules`, `severity`, `paths` to the h-3 contract, ACs and REQ-PRODUCT-023, or record a Rand-ruled decision row narrowing #201 (reword Goal and closure table, comment on #201).

## Detailed Findings
### Important
- **PH-I1** `epic lint-phase-h; beads lint-h-1,2,3,4,5,7 (owned_paths)` — Plan never mentions open stack #186 (#155 function-length lint). Grep of the epic and all sprint beads for #186/#155/function-length: no match, while owned paths (crates/sc-lint/src/{cli,command,config,workflow,render,entry,lib,tests}.rs, boundary lib/analysis/tests.rs) overlap. No landing order or merge-forward point is named (CLAUDE.md forbids gh stack sync/rebase). (req ATM-QA-002, boundary RBQA-F004, arch ARCH-005) _Fix:_ State whether #186 lands on develop before h-1 is cut, or name the merge-forward event; add a function-length compliance check to affected sprint ACs.
- **PH-I2** `epic lint-phase-h AC 1 and 'Approvals required before dispatch'; ADR-015/ADR-016 line 9` — Epic AC 1 requires ADRs 'Status Accepted' with an 'Approved by' row and 'no sprint dispatched before that', which the ruling removed as a gate and which can never pass against Pending ADRs. ADR-015/016 headers also keep 'Approved by user (Rand Lee), 2026-08-29' beside a Proposed amendment (ADR-005/012 have no such row). Pending status itself is not a finding. (req ATM-QA-003, boundary F001/F003, arch ARCH-008) _Fix:_ Reword AC 1 to the ruled state; relabel the old row 'Original decision approved by'.
- **PH-I3** `beads lint-h-5, h-12, h-13, h-14; crates/sc-lint/src/config.rs (915 lines), workflow.rs (604), command.rs (606), boundary graph/build.rs (961), lib.rs (638), analysis.rs (624)` — No sprint carries a file-size budget or names new modules for the step/platform plan, batch executor and rule-catalog code; config.rs and build.rs are already near the 1000-line RULE-003 limit. (arch ARCH-001) _Fix:_ Add an AC per sprint: no touched file exceeds 1000 non-test lines; name the new modules.
- **PH-I4** `beads lint-h-1, h-3, h-5, h-7, h-12 (subprocess tests)` — Subprocess tests spawning the sc-lint binary do not require ATM_HOME/ATM_CONFIG_HOME isolation or per-command env (RULE-011). (arch ARCH-002) _Fix:_ One shared tests/support module that scrubs ATM_*; AC requiring its use.
- **PH-I5** `beads lint-h-3, lint-h-7, lint-h-12` — Analyzer scaffolding (AnalyzeOptions, ANALYZER_NAME, KNOWN_KEYS, RuleCatalog impl, Config error, main.rs exit mapping) is repeated across three crates; h-12 ProvisionAdapter mirrors SystemAdapter methods. (arch ARCH-003) _Fix:_ Define shared analyzer args/exit mapping once; have ProvisionAdapter extend SystemAdapter.
- **PH-I6** `beads lint-h-4 (A-08/A-09 owner_crate_path check removal, allowed_dev_dependencies, [extensions]); lint-h-14 AC16` — Both sprints loosen or reshape enforcement layers justified only by pending A-rows; boundary records and oracle/lint_boundaries.py coverage are not shown updated first. (arch ARCH-004) _Fix:_ Update boundary requirements/enforcement-model/schema records in the same plan; confirm no coverage reduction.
- **PH-I7** `bead lint-h-7 (dispatch.rs RULE_FILTER_ERROR_MARKER), epic A-19/A-20, lint-h-3 AC15/36/36b` — Usage (exit 2) vs config (exit 3) for a delegated backend is decided by substring-matching backend stderr, and the marker is duplicated between dispatch.rs and sc-lint-schema; a human-readable message becomes a cross-crate contract with two owners. (rust-best-practices RBP-F001, boundary RBQA-F006, arch ARCH-006) _Fix:_ Use a stable machine-readable code or one shared constant exported from sc-lint-schema, with a drift test; record the brittleness in A-19.

### Minor
- **PH-M1** `lint-h-7 lines 24-25; lint-h-13 line 25; lint-h-11 lines 1638,1674; lint-h-4` — Sprint beads describe Pending ADR/REQ text as 'approved' (e.g. 'approved in PR #206', 'approved ADR-016 How To Extend text'); should read Pending. (req ATM-QA-004, boundary F002, arch ARCH-009, RBP) _Fix:_ Reword; re-grep all beads for 'Approved' next to ADR-005/012/015/016.
- **PH-M2** `epic shared prerequisites; lint-h-5, lint-h-7` — Epic cites h-5 AC 1-38 (bead has AC1-AC39) and h-7 T1-T22 (bead has T1-T23); AC36-AC39 unmapped in closure table. (req ATM-QA-005) _Fix:_ Update ranges and mapping.
- **PH-M3** `epic issue closure table; lint-h-11 AC4,10,11,12,18,20,21,27,28,33` — Ten h-11 ACs have no issue mapping in the closure table. (req ATM-QA-006) _Fix:_ Map or mark integration-only.
- **PH-M4** `lint-h-13 D1 vs H13-4` — 'six rule sentences' vs 'five rules plus the cargo target-dir sentence'. (req ATM-QA-007) _Fix:_ State the count identically.
- **PH-M5** `epic decisions table; lint-h-2, h-5, h-11` — Issue asks reshaped (always-install vendored action despite ci.enabled=false, `just --dump` vs `--list`, exit 7 vs 10, groups/profiles/platform shapes) without confirmed A-xx rows or per-issue comments. (req ATM-QA-008) _Fix:_ Add decision rows; comment on each issue.
- **PH-M6** `lint-h-4 Ordering` — h-4 (#203) has no edge to h-3 (#201) and no sentence explaining independence. No edge recommended. (boundary F005, req ATM-QA-009) _Fix:_ Add one sentence on independence.
- **PH-M7** `docs/project-plan.md:101; docs/architecture.md` — Neither baseline mentions phase-h, 0.6.1 or its sprints. (req ATM-QA-010) _Fix:_ Add a phase-h pointer section.
- **PH-M8** `beads lint-h-1, h-3, h-5, h-12, h-13 fixtures` — No AC bars production team/identity literals in subprocess fixtures (RULE-008); AC4 grep too narrow. (arch ARCH-007) _Fix:_ Use shared constants; widen grep.
- **PH-M9** `lint-h-3 (RuleCatalog), lint-h-12, lint-h-5` — `pub trait RuleCatalog` has no sealed/open decision and is not object-safe (state static-dispatch-only); target triples, runner path and selector names stay raw String instead of validated newtypes. (RBP-F002..F006) _Fix:_ Record decisions; introduce TargetTriple/SelectorName/RelativeSlashPath.
- **PH-M10** `validate-plan (.claude/skills/atm-beads/scripts/validate-plan)` — Live `validate-plan --phase h` exits 2 (no `.atm-bd/phase-h.toml` in main checkout; `origin/integrate/phase-h` absent); `--file phase-h.jsonl --index` crashes `KeyError: 'id'` at line 72; `--ci` passes. The expected exit 5 on folded beads lint-h-6/8/9/10 did not reproduce (not reachable). Folded beads are closed with 'task h-N:' titles. Beads wiring for all depends_on edges verified correct. (validate-plan check) _Fix:_ Re-run validate-plan once integrate/phase-h exists; resolve lint-wf-validate-plan-folded disposition.

## Merge Readiness
- Status: **NOT READY**
- Reason: 1 blocking, 7 important, 10 minor. Deliverable completion 12/20 (60%) per req-qa. Coverage: all of #193-#204 map to sprints with ACs; #201 is only partly covered (PH-B1). Ordering (be-1/2/3/4, #201 before #202/#203, #200 early) and the jsonl/beads dependency wiring verified. Pending ADRs 005/012/015/016 not treated as a finding; only approval wording is reported. CI on #206 passes; branch is 1 merge (#207 plan-anchoring skill) behind develop, so use the three-dot diff and merge develop forward before adoption. Review limits: parts of beads h-2, h-3, h-5, h-11, h-12 were checked by AC heading only; service-hardening and schema-reviewer reviewers not run (not in standing sc-lint set / no such agent); origin/integrate/phase-h does not exist so plans/phase-h was substituted.

## Next Action
- Action: Fix all findings in plan content (beads and #206) on a new layer, triage via /triaging-findings, then request QA-2.
- Owner: team-lead
