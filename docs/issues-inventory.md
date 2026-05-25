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

Phase A planning did not introduce a new ATM workflow, QA-routing scheme, or
triage-protocol change.

Current live governing process documents remain:

- `docs/team-protocol.md`

Phase `B` now contains the explicit follow-on process work that was previously
reserved for a later phase:

- `B.4`
  - triage-first QA routing
  - QA-1-only default `rust-best-practices` usage
  - TODO scan and carry-forward triage automation
  - see `docs/sc-lint/sprint-B4.md`

The B.4 authoritative routing surfaces are now:

- `.claude/agents/quality-mgr.md`
- `.claude/agents/qa-triage.md`
- `.claude/skills/codex-orchestration/SKILL.md`
- `.claude/skills/triaging-findings/SKILL.md`
- `.claude/skills/todo-triage/SKILL.md`
- `.claude/skills/codex-orchestration/qa-template.xml.j2`
- `.claude/skills/codex-orchestration/fix-assignment.xml.j2`
- `scripts/find_todos.py`
- `scripts/triage_carry_forward.py`

These are now the authoritative default by plan status, not an imported
placeholder awaiting later hardening.

## Distribution Planning Scope

The current released Homebrew automation still reflects the boundary-only
stopgap path. Phase `B` contains the explicit follow-on distribution planning
needed to move from that stopgap to the full top-level install path:

- `sprint-B-homebrew`
  - planned primary `brew install randlee/tap/sc-lint` surface
  - explicit disposition for `sc-lint-boundary.rb`
  - full release-manifest and tap-update planning
  - see `docs/sc-lint/sprint-B-homebrew.md`

## Phase C Planning Note

Phase `C` is now the queued interface-versioning and shared portability
follow-on planning line:

- `C.1`
  - ownership/policy/baseline definition
- `C.2`
  - generated HTML/XHTML/JSON artifact publication planning
- `C.3`
  - hard-fail version-gate planning
- `C.4`
  - consumer-adoption document and repo-local skill design
- `C.5`
  - minimal marketplace publication planning for the adoption skill
- `C.6`
  - production path-literal portability parity
- `C.7`
  - broad environment-variable portability
- `C.8`
  - shell invocation portability
- `C.9`
  - structural cross-platform `cfg` parity enforcement

Current status:
- no dedicated boundary record or `boundaries/planning.toml` entry is required
  yet for `sc-lint-version`
- `C.1` has now committed the planned form-factor and invocation path:
  - dedicated workspace crate: `sc-lint-version`
  - top-level command path: `sc-lint check interfaces`
- dedicated crate-boundary records remain future implementation work after the
  Phase `C` planning line rather than current planning prerequisites
