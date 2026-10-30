# Intent - phase-m

Written 2026-10-30 by flint (draft for Rand's confirmation). One page at most. This is the "why" that frames every anchor beneath it; Jev questions are judged against anchors, not against this document, so keep it short and let the anchors carry the testable facts.

## What this plan is for

Rand named four atm-core items as P1 for sc-lint: "these are the p1 items from atm-core", and asked for "anchoring statements for each of the items listed from atm-core". Each item is a place where atm-core, on released sc-lint 0.8.2, still keeps a lint exclusion, a local checker or Justfile glue because sc-lint cannot yet express what it needs. They are #786 and #720 (`PORT-010`), #198 (consumer setup hook and step runner) and #136 (manifest dependency ownership). Phase M closes those gaps in sc-lint so atm-core can delete its workarounds. sc-lint remains a general-purpose tool: atm-core supplies the failing cases, and its repository-specific policy stays in consumer configuration.

## What success looks like

- On a release that includes Phase M, atm-core can remove its `PORT-010` exclusions for the #786 paths and the #720 crate-root shape, while real cross-platform parity gaps are still reported (ANC-001, ANC-002).
- atm-core can drop its local source-only ownership checker, because `global_dependency_ownership` offers a source-only scope (ANC-006).
- Each of atm-core's three `[boundaries]` keys has a recorded coverage verdict, and every remaining gap has an issue (ANC-007).
- Conditional on a new ADR approved by Rand that supersedes ADR-020 decisions 9 and 13 and REQ-CLI-021: atm-core can delete its `_py`, `sc_lint_clippy_prefix`, `_sc-lint-clippy` and `_sc-lint-py` Justfile glue for #198 (ANC-003 to ANC-005).
- Each sprint traces to its GitHub issue and carries a deterministic regression for the reported consumer failure.

## What is deliberately out of scope

- Suppressing findings by blanket exclusion, by disabling a rule, or by widening allowed roots for atm-core paths.
- #198 work (ANC-003 to ANC-005) before the superseding ADR is Accepted. ANC-005's design also waits for atm-core's input, and so does the #786 per-path classification.
- Other atm-core gaps that are not on the P1 list (for example #783, #784, #195 and #385), the 0.8.3 release-platform work, and any new process or tooling changes.

## Anchors

Anchors for this plan live in `anchors.json` (record of truth) and are rendered to `anchors.md`. Requirements and ADRs that govern this plan are listed there too, under their own IDs. All anchors are `proposed` until Rand confirms them; none has yet been through the Jev rubric.
