# Intent - phase-i

Written 2026-10-10 by flint for Rand Lee. One page at most. This is the "why"
that frames every anchor beneath it; Jev questions are judged against anchors,
not against this document, so keep it short and let the anchors carry the
testable facts.

## What this plan is for

Phase-i closes the gaps the atm-core team filed as GitHub issues #193-#204.
Each issue is lint behavior atm-core has today that it would lose by migrating
to `sc-lint`. Every fix lands at the appropriate layer of the layered
architecture in [docs/architecture.md](../../architecture.md#layered-architecture)
(ANC-001).

## What success looks like

- atm-core can replace its own lint orchestration with `sc-lint` without
  losing any behavior named in #193-#204.
- Each fix is a targeted addition in one layer, sized as one small PR per
  issue.

## What is deliberately out of scope

- Redesign. Any substantial redesign, including the departures listed in
  docs/architecture.md "Where the current code departs from the layering", is
  discussed with Rand before it is documented or planned.
- Anything not required to address #193-#204.

## Anchors

Anchors for this plan live in `anchors.json` (record of truth) and are
rendered to `anchors.md`. Requirements and ADRs that govern this plan are
listed there too, under their own IDs.
