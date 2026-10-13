# Intent - phase-j: automated sc-compose upgrade

Written 2026-10-12 by Rand Lee (recorded by team-lead). One page at most. This is the "why" that frames every anchor beneath it; Jev questions are judged against anchors, not against this document, so keep it short and let the anchors carry the testable facts.

## What this plan is for

Automate the upgrade of sc-compose with a small set of steps that form a
mini-phase. The upgrade is a formula so the planning work is done ahead of
time: a run pours the formula, then builds and executes the sprint steps in
order without planning anything new.

The whole process runs unattended. Every escalation goes to
`omega-prime@hermes`.

## What success looks like

- Triggering the upgrade pours the formula and runs its steps in order to a PR
  with no user input.
- Every decision point and escalation reaches `omega-prime@hermes`.

## What is deliberately out of scope

- Planning during a run: the formula is the plan.

## Anchors

Anchors for this plan live in `anchors.json` (record of truth) and are rendered to `anchors.md`. Requirements and ADRs that govern this plan are listed there too, under their own IDs.
