# Phase M planning gate, round 1 (lint-pcq2)

Run 2026-10-30 by flint at plan head 03bf1d60. Model `jev-1.14.0`, threshold 0.6. Each sprint container bead (lint-m-1, m-2, m-3, m-4, m-5a, m-5b) was composed with `compose-item --for plan` (title, description, acceptance; no design or notes). It was judged once with `judge --gate plan` against the 15 locked, tuned anchors: one anchor and one bead per Jev request, never the whole plan. Every anchor's tuned `question_overrides` applied (`overridden: true`); no item was truncated or shortened. The run was inline, not through the anchor-plan-review agent, because subagents cannot inherit the remote-judge opt-in; team-lead approved this (option a).

**Not yet gated:** ANC-018, ANC-020 and ANC-021 (pending definition of the approved exception path) were excluded.

**Result: no bead passes, and three locked anchors are uncovered (ANC-023, ANC-024, ANC-026).** No plan or anchor was edited; triage-apply-rerun is for team-lead with clint.

## Verdicts

S = satisfied, na = not_applicable, NA! = not_addressed, U! = underspecified.

| Bead | 014 | 016 | 019 | 022 | 023 | 024 | 025 | 026 | 027 | 028 | 029 | 030 | 031 | 032 | 033 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lint-m-1 | NA! | na | na | na | na | na | na | U! | na | na | na | na | na | na | na |
| lint-m-2 | na | na | na | na | na | na | S | na | NA! | na | S | na | NA! | na | na |
| lint-m-3 | na | na | S | S | U! | NA! | na | na | S | S | na | na | U! | S | S |
| lint-m-4 | na | S | na | na | na | na | NA! | na | U! | na | NA! | S | S | na | na |
| lint-m-5a | NA! | na | na | na | na | na | na | NA! | na | na | na | na | na | na | na |
| lint-m-5b | S | U! | na | na | na | na | na | NA! | U! | na | na | na | U! | na | na |

## Failing cells: weak spot (`to_fix`)

Answers are relevant / addressed / contradicted with confidence. The `where` is the bead's own sentence; the `change` is a suggestion for triage, not an applied fix.

| Bead · anchor | Verdict (leaning, min conf) | Answers | where | change |
|---|---|---|---|---|
| m-1 · ANC-026 | underspecified (contradicted, 0.33) | yes 1.0 / yes 0.33 / yes 0.33 | "Extend PORT-010's free-function AND inherent-method companion predicates to exact cfg(not(unix)) production siblings in the same lexical scope" and "Pending ADR-021 decision 5 requires Rand approval before implementation." | State flatly that a cfg(unix) item with a same-name cfg(not(unix)) sibling in the same module, inherent impl or crate root produces no PORT-010 finding. The "production" qualifier and ADR gate read as conditions the anchor does not have. m-1 is the only S3 owner, so this cell decides coverage. |
| m-1 · ANC-014 | not_addressed (0.85) | yes 1.0 / no 0.96 / no 0.85 | "m-5a/m-5b own S1/S2/S4-S8 and final #786 closure." | m-1 is a layer, not the zero-false-positive outcome. Drop ANC-014 from m-1's Active Trace, or state m-1's share (the six S3 occurrences report nothing). |
| m-2 · ANC-027 | not_addressed (0.79) | yes 0.79 / no 1.0 / no 0.98 | null | No bead change: setup.pre is not m-2's work. ANC-027's scope ("edits ScLintConfigFile or consumer config parsing") catches any config key. Anchor-scope triage. |
| m-2 · ANC-031 | not_addressed (0.73) | yes 0.89 / no 0.86 / no 0.73 | "The three-key parity audit belongs to m-4." | No bead change needed beyond that deferral: relevance error or scope. |
| m-3 · ANC-024 | not_addressed (leaning contradicted, 0.48) | yes 1.0 / no 0.61 / yes 0.48 | "sc-lint-python -m sc_lint.<module> and python -m sc_lint.<module> use kit Python by default" | Say which .bootstrap-venv interpreter each step currently run by _py, sc_lint_clippy_prefix and _sc-lint-clippy gets: plain python steps via runner.python, and both clippy steps via PATH/PYO3_PYTHON. If any of those steps are sc_lint.* helpers that move to kit Python (ANC-022), that is a real ANC-022/ANC-024 tension to raise with Rand. |
| m-3 · ANC-023 | underspecified (not_addressed, 0.30) | yes 1.0 / no 0.30 / no 0.76 | "Run pre once from repo root before bootstrap provisioning" | Say that sc-lint setup runs the consumer's pre ["just","bootstrap"], which builds .bootstrap-venv, so atm-core no longer needs its own glue to do it. |
| m-3 · ANC-031 | underspecified (not_addressed, 0.48) | yes 0.48 / no 0.91 / no 0.97 | null | Not m-3's work; low relevance. Scope triage. |
| m-4 · ANC-025 | not_addressed (0.97) | yes 0.98 / no 0.97 / no 0.97 | "m-2 owns source-only implementation." | m-4 audits and implements nothing, so the anchor's scope ("changes global_dependency_ownership … or their config keys") is being read as covering an audit of them. Scope or relevance triage. |
| m-4 · ANC-029 | not_addressed (0.88) | yes 0.98 / no 0.88 / no 0.97 | "m-2 owns source-only implementation." | Same as ANC-025. |
| m-4 · ANC-027 | underspecified (not_applicable, 0.53) | no 0.53 / no 1.0 / no 1.0 | null | Not m-4's work. ANC-027 scope triage. |
| m-5a · ANC-014 | not_addressed (0.33) | yes 1.0 / no 1.0 / no 0.33 | "no claim of zero until m-5b and S8 are complete" / "S8 public helpers remain reported at this layer" | Layer, not outcome. Drop from m-5a's trace, or state m-5a's share. The 0.33 on contradicted comes from S8 staying reported at this layer. |
| m-5a · ANC-026 | not_addressed (0.64) | yes 0.98 / no 0.94 / no 0.64 | "S3 is consumed from m-1, not reimplemented." | Deferral to m-1. Relevance or scope triage (ANC-026's scope covers port010 fixtures, which m-5a extends). |
| m-5b · ANC-026 | not_addressed (0.76) | yes 0.98 / no 0.82 / no 0.76 | null | m-5b asserts zero findings on all 18 paths, which includes S3. Either state that S3 pairs stay unreported in the final verification or treat it as a scope match. |
| m-5b · ANC-016 | underspecified (not_applicable, 0.52) | no 0.52 / no 0.97 / no 0.97 | "Update docs/table and mark #720/#786 closure coverage honestly." | "table" reads as the parity table; name it the #786 47-item table. |
| m-5b · ANC-027 | underspecified (not_addressed, 0.56) | yes 0.60 / no 1.0 / no 1.0 | "using a temporary consumer config at pinned 6ffaa97be" | Not setup.pre. ANC-027 scope triage. |
| m-5b · ANC-031 | underspecified (not_addressed, 0.30) | yes 0.30 / no 0.89 / no 0.97 | null | Not m-5b's work. Scope triage. |

## Patterns for triage

1. **Broad anchor scopes (anchor defects):** ANC-027 is relevant to m-2, m-4 and m-5b, and ANC-031 to m-2, m-3 and m-5b, though none of those do the work. ANC-025 and ANC-029 are relevant to m-4's audit. These are 8 of 16 failing cells. A scope change is `update --scope` on a locked anchor, so it needs Rand's approval under the amend path.
2. **Layered sprints (014, 026):** m-1, m-5a and m-5b each carry part of the #786 outcome and defer the rest. Jev reads the deferral as not addressing. A plan fix (narrow each Active Trace, or state each layer's share) or a question fix (tuning) would resolve it; deciding which is triage.
3. **Uncovered anchors (plan gaps):**
   - ANC-026: m-1 is the only S3 owner and is underspecified, leaning contradicted.
   - ANC-023 and ANC-024: only m-3 carries them, and it is not explicit about .bootstrap-venv / just bootstrap.
   - The ANC-024 / ANC-022 interaction (sc_lint.* helpers on kit Python vs the .bootstrap-venv interpreter for the steps the recipes run today) may be a real tension, not wording.
4. **Stale trace IDs in the beads:**
   - Active Trace and Goal lines still cite retired IDs: GH-786.S3 (m-1), GH-136.src-only.2 (m-2), GH-198.setup-pre.2/runner-argv.2/runner-path.2/export-env.3 (m-3) and GH-136.audit-verify/audit-gaps (m-4).
   - They also cite "draft ANC-025" and "draft ANC-016" (m-2, m-4).
   - Jev does not see IDs as authority, but the beads should carry ANC-026..033.

## Low-confidence diagnostics (lint-e9jk)

Every cell with min confidence below 0.6 is a failing cell above. No satisfied or not_applicable cell has a role below 0.6, so the 74 satisfied/not_applicable cells (of 90) are firm. Nothing here is a production PASS: no bead passes.

## Coverage (script)

## Anchor coverage (plan)

| Anchor | lint-m-1 | lint-m-2 | lint-m-3 | lint-m-4 | lint-m-5a | lint-m-5b | Covered |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ANC-014 | not addressed | not applicable | not applicable | not applicable | not addressed | satisfied | yes |
| ANC-016 | not applicable | not applicable | not applicable | satisfied | not applicable | underspecified | yes |
| ANC-019 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-022 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-023 | not applicable | not applicable | underspecified | not applicable | not applicable | not applicable | no |
| ANC-024 | not applicable | not applicable | not addressed | not applicable | not applicable | not applicable | no |
| ANC-025 | not applicable | satisfied | not applicable | not addressed | not applicable | not applicable | yes |
| ANC-026 | underspecified | not applicable | not applicable | not applicable | not addressed | not addressed | no |
| ANC-027 | not applicable | not addressed | satisfied | underspecified | not applicable | underspecified | yes |
| ANC-028 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-029 | not applicable | satisfied | not applicable | not addressed | not applicable | not applicable | yes |
| ANC-030 | not applicable | not applicable | not applicable | satisfied | not applicable | not applicable | yes |
| ANC-031 | not applicable | not addressed | underspecified | satisfied | not applicable | underspecified | yes |
| ANC-032 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |
| ANC-033 | not applicable | not applicable | satisfied | not applicable | not applicable | not applicable | yes |

Uncovered anchors (no item satisfies them): ANC-023, ANC-024, ANC-026

