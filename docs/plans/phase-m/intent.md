# Intent - phase-m

Written 2026-10-30 by flint (draft for Rand's confirmation). One page at most. This is the "why" that frames every anchor beneath it; Jev questions are judged against anchors, not against this document, so keep it short and let the anchors carry the testable facts.

## What this plan is for

Rand's words (approved 2026-10-30): "atm-core should be able to delete its workarounds: when sc-lint is done, every item on its P1 list passes as the GitHub issue describes, with nothing hidden."

The P1 list is #786 and #720 (`PORT-010`), #198 (consumer setup hook and step runner) and #136 (manifest dependency ownership). On released sc-lint 0.8.2, atm-core keeps a lint exclusion, a local checker or Justfile glue for each. sc-lint remains a general-purpose tool: atm-core supplies the failing cases, and its repository-specific policy stays in consumer configuration.

## What success looks like

- On a release that includes Phase M, atm-core can remove its `PORT-010` exclusions for the #786 paths and the #720 same-name sibling shape in every scope, while real cross-platform parity gaps are still reported (ANC-014, ANC-018, ANC-020, ANC-021, GH-786.S3).
- atm-core can drop its local source-only ownership checker, because `global_dependency_ownership` offers a source-only scope (ANC-012, GH-136.src-only.2).
- Each of atm-core's three `[boundaries]` keys has a recorded coverage verdict, and every remaining gap has an issue (ANC-016, GH-136.audit-verify, GH-136.audit-gaps).
- Conditional on a new ADR approved by Rand that supersedes ADR-020 decisions 9 and 13 and REQ-CLI-021: atm-core can delete its `_py`, `sc_lint_clippy_prefix`, `_sc-lint-clippy` and `_sc-lint-py` Justfile glue for #198 (ANC-010, ANC-011, ANC-019, ANC-022, GH-198.*).
- Each sprint traces to its GitHub issue and carries a deterministic regression for the reported consumer failure.

## What is deliberately out of scope

- Suppressing findings by blanket exclusion, by disabling a rule, or by widening allowed roots for atm-core paths.
- #198 work (ANC-010, ANC-011, GH-198.*) before the superseding ADR is Accepted. Rand ruled (2026-10-30) that the #198 runner.export_env (PYO3_PYTHON) need is in scope (GH-198.export-env.3) and that pub POSIX-only helpers (S8) are exempt. His later rulings (2026-10-30) require an approved exception path for any knowing PORT-010 opt-out, so S8 is exempt through that path, not by blanket rule (ANC-018, ANC-021), and a consumer method to override behavior such as the sc_lint.* helper interpreter (ANC-019).
- Other atm-core gaps that are not on the P1 list (for example #783, #784, #195 and #385), the 0.8.3 release-platform work, and any new process or tooling changes.

## Anchors

Anchors for this plan live in `anchors.json` (record of truth) and are rendered to `anchors.md`. Requirements and ADRs that govern this plan are listed there too, under their own IDs. Per-item anchors quote Rand's approved sentences; GH-* records carry the GitHub issue acceptance text they cite. All are `proposed` until Rand confirms them.
