# Issues Inventory

This document tracks the current known issue state for the `sc-lint` planning
surface.

## Current Status

As of the Phase A hardening pass, there are no open blocking plan-integrity
findings in the documented A.1a through A.8 sprint sequence once the current
hardening changes are applied.

## Active Non-Blocking Notes

1. `feature/sprint-A1` remains a historical local worktree name from before the
   A.1 split.
   Current status:
   - it is not part of the canonical Phase A sprint naming
   - no rename or recreation is required unless that worktree is reactivated
   - `A.1a` and `A.1b` are the authoritative sprint identifiers in all plan
     docs

2. `xwin` remains a local preflight capability, not a CI-parity requirement.
   Current status:
   - `fast` excludes `xwin`
   - `full` may include `xwin check` and `xwin clippy` when available
   - `ci` excludes `xwin`
   - real Windows CI remains the authoritative release gate

## Process / QA / Triage Scope

Phase A planning does not introduce a new ATM workflow, QA-routing scheme, or
triage-protocol change. The current governing process documents remain:

- `docs/team-protocol.md`

If a later phase changes team routing or QA handoff behavior, add that work as
an explicit phase or sprint deliverable rather than leaving it implicit.
