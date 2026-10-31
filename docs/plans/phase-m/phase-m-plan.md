# Phase M Plan — atm-core Tool Fixes

Rand approved the four-sprint cut on 2026-10-31. The authoritative scope,
design and acceptance live on the sprint beads. This page describes their
relationships; the issue closure checks live on the closure beads.

## Authority

| Document | Status | Dispatch effect |
| --- | --- | --- |
| [ADR-022](../../sc-lint/adr/ADR-022-tools-and-consumer-configuration.md) | Pending | lint-t6i7 blocks m-4 only |
| [ADR-010 amendment 2026-10-30](../../sc-lint/adr/ADR-010-portability-scope-and-parity.md) | Pending | lint-m-adr010 blocks m-1 |
| ADR-021 | Withdrawn | none |
| REQ-PRODUCT-004AA, REQ-SCB-019, REQ-CLI-021 | Accepted | existing contracts remain effective |
| [v3 anchors](v3/anchors.json) | Eight kept locked anchors | sealed planning questions unchanged |

## Sprints

| Sprint bead | Stack / layer | Owner | Issues and scope | Anchors |
| --- | --- | --- | --- | --- |
| lint-m-1 port010-same-name-siblings | A1 | clint | #720, #786 S3 same-name cfg(not(unix)) siblings and S6 out-of-line Unix-only modules | ANC-003, ANC-006, ANC-007, ANC-009 |
| lint-m-4 port010-site-allow | B1 | cfast | #786 S8 and every other shape through reasoned item allowances; allowed entries in reports | ANC-010, ANC-009 |
| lint-m-2 manifest004-local-module-paths and cycle paths | C1 | clint | #136 local-module paths; #385 SCB-CYCLE-001 path exclusions | ANC-013 |
| lint-m-3 lint-step-env-cwd and visible output | D1 | cfast | #198 lint env/cwd; #195 both failed-step streams and every step's output log | ANC-011, ANC-012 |

m-5a, m-5b, m-5c, m-7 and m-8 and their dev/sanity/QA groups are superseded.
m-6 was already folded into the former plan and stays closed. There is no
caller index, finding inventory, burndown, hash test, extraction script or
adoption-bead deliverable. m-1 updates the existing atm_core_* fixture exact
lists and replaces the crate-root-main limitation regression.

## Stacks And Shared Files

```text
develop
├── A: m-1
├── B: m-4
├── C: m-2
└── D: m-3
```

Each stack is one layer rooted on develop. Stack operations follow
[CLAUDE.md](../../../CLAUDE.md); the lead is the stack writer. The sprint
beads have no cross-stack must_follow edges. Capacity is two owners:
clint runs m-1 then m-2; cfast runs m-4 then m-3. These are owner schedules,
not new dependency gates. Only m-1 waits for
lint-m-adr010 and only m-4 waits for lint-t6i7. m-2 is governed by REQ-SCB-019 and ADR-020, with the SCB-CYCLE-001
amendment Pending Rand acceptance before dispatch. m-3 implements existing
ADR-016, REQ-CLI-023 and ADR-020 decisions 7/9. ADR-022 is context for m-2
and m-3, not their dispatch gate. Plan review and the planning
gate supply evidence; lint-phase-m-plan-qa blocks no sprint or dev bead.

m-1 and m-4 share portability's parity.rs, lib.rs and tests.rs. m-4 lands
first; whichever lands second receives develop merged forward by the stack
writer. This merge is a stack operation, not another sprint. m-2's
manifest-policy/lib.rs changes and m-4's boundary graph directive match are
separate edits; the stack writer resolves any integration conflict.

## Issue Closure

Each sprint QA group confirms that delivery matches the issue's **Planned
delivery (Phase M)** comment and posts its approval on that issue. Sprint QA
covers #720/#786 for m-1, #786 for m-4, #136/#385 for m-2 and #198/#195 for
m-3. It does not close the issues ahead of the closure evidence.

Phase M is complete when the standalone lint-phase-m-done gate closes.
That gate depends only on the five closure beads below. The phase epic has
no closure edges: its sprint parent-child links would create a dependency
cycle. The whole overdue-issue epic lint-8h3t and other release/platform
issues stay outside this phase. lint-8h3t.3 (#720) is merged into lint-8h3t.2 (#786 and #720).

| Issues | Closure bead | Check / prerequisites |
| --- | --- | --- |
| #786, #720 | lint-8h3t.2 | After 0.8.3 ships with m-1 and m-4: atm-core at its head, all 18 PORT-010 exclusions removed, site allowances on remaining items; zero PORT-010 findings over those paths and a reason on every allow. The allowance count is measured from that run, not the estimate. Depends on m-1, m-4 and release bead lint-8h3t.1, plus atm-core sign-off. |
| #136 | lint-8h3t.4 | After m-2: atm-core at head with ownership restored and owner tests/ roots configured reports no SCB-MANIFEST-004 findings; an injected non-owner mutation still reports. |
| #198 | lint-8h3t.5 | After m-3: bare sc-lint runs clippy with a step PYO3_PYTHON env value on a host whose PATH python is older than 3.11. |
| #385 | lint-8h3t.8 | Cite m-2's boundary_cycle_exclude_paths_by_rule result: two cycles in two files, only the excluded cycle disappears. Do not rerun the sprint test. |
| #195 | lint-8h3t.9 | Cite m-3's consumer_failed_step_prints_both_streams and consumer_log_root_writes_every_step results: both failed-step streams visible; every step writes <log-root>/steps/<step>.log. Do not rerun the sprint tests. |

The existing logger root meaning of --log-root stays intact. Consumer
adoption of m-4 adds sc-lint-attributes to Cargo dependencies and each crate's
boundary allowed_dependencies; every reason names the item's actual cause.
The attribute remains a no-op for behavior/public API and builds through the
existing stable Linux, macOS and Windows CI matrix.

## Determinism

Gating tests use committed fixtures and controlled inputs, with no sleeps,
retries, network, clock, host-dependent paths, races or event-loop ordering.
Consumer runs belong to closure checks; sprints end with just lint and just
test. No extra adoption run or CI gate is introduced.

## Pending Decisions For Rand

1. Accept ADR-022 (lint-t6i7).
2. Accept the ADR-010 amendment (lint-m-adr010).
3. S6 public scope: whether pub modules/items are covered or require a site
   allowance. ANC-006/007 remain locked as written until Rand decides.
4. Accept the ADR-020 Amendment 2026-10-31 extending path-less exclusions to
   SCB-CYCLE-001 before m-2 dispatch.
