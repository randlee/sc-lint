# Intent - phase-m

Written 2026-10-30 by flint (draft for Rand's confirmation). One page at most. This is the "why" that frames every anchor beneath it; Jev questions are judged against anchors, not against this document, so keep it short and let the anchors carry the testable facts.

## What this plan is for

Rand's words (approved 2026-10-30): "atm-core should be able to delete its workarounds: when sc-lint is done, every item on its P1 list passes as the GitHub issue describes, with nothing hidden."

The P1 list is #786 and #720 (`PORT-010`), #198 (consumer setup hook and step runner) and #136 (manifest dependency ownership). On released sc-lint 0.8.2, atm-core keeps a lint exclusion, a local checker or Justfile glue for each. sc-lint remains a general-purpose tool: atm-core supplies the failing cases, and its repository-specific policy stays in consumer configuration.

## What success looks like

- On a release that includes Phase M, atm-core can remove its `PORT-010` exclusions for the #786 paths and the #720 same-name sibling shape in every scope, while real cross-platform parity gaps are still reported (ANC-014, ANC-018, ANC-020, ANC-021, ANC-026).
- atm-core can drop its local source-only ownership checker, because `global_dependency_ownership` offers an opt-in source-only scope, with all-files checking still the default (ANC-025, ANC-029).
- Each of atm-core's three `[boundaries]` keys has a recorded coverage verdict, and every remaining gap has an issue (ANC-016, ANC-030, ANC-031).
- Conditional on a new ADR approved by Rand that supersedes ADR-020 decisions 9 and 13 and REQ-CLI-021: atm-core can delete its `_py`, `sc_lint_clippy_prefix`, `_sc-lint-clippy` and `_sc-lint-py` Justfile glue for #198 (ANC-023, ANC-024, ANC-019, ANC-022, ANC-027, ANC-028, ANC-032, ANC-033).
- Each sprint traces to its GitHub issue and carries a deterministic regression for the reported consumer failure.

## What is deliberately out of scope

- Suppressing findings by blanket exclusion, by disabling a rule, or by widening allowed roots for atm-core paths.
- #198 work (ANC-023, ANC-024, ANC-019, ANC-022, ANC-027, ANC-028, ANC-032, ANC-033) before the superseding ADR is Accepted. Rand ruled (2026-10-30) that the #198 runner.export_env (PYO3_PYTHON) need is in scope (ANC-032) and that pub POSIX-only helpers (S8) are exempt. His later rulings (2026-10-30) require an approved exception path for any knowing PORT-010 opt-out, so S8 is exempt through that path, not by blanket rule (ANC-018, ANC-021), and a consumer method to override behavior such as the sc_lint.* helper interpreter (ANC-019).
- Other atm-core gaps that are not on the P1 list (for example #783, #784, #195 and #385), the 0.8.3 release-platform work, and any new process or tooling changes.

## Anchors

Anchors for this plan live in `anchors.json` (record of truth) and are rendered to `anchors.md`. Requirements and ADRs that govern this plan are listed there too, under their own IDs. Per-item anchors quote Rand's approved sentences or the GitHub issue acceptance text they cite. ANC-026 to ANC-033 were registered as GH-* records and re-registered on 2026-10-30 under ANC IDs only, because the question tuner accepts ANC IDs; their quotes, statements and fixtures did not change. Rand locked the 18 active records on 2026-10-30: "we can lock the anchors, but if tuning requires updating them, they will need to be changed".

**Amend path for a locked anchor.** A change to a locked statement is a new version that needs Rand's approval. It invalidates every earlier gate and tuning run for that anchor, which must be re-run against the new version. A change to the quote (his words) is a new anchor that retires the old one (`retire --replaced-by`).

ANC-018, ANC-020 and ANC-021 are locked but pending definition: Rand has not yet defined the "approved exception path" they rest on. Their summaries say so.
