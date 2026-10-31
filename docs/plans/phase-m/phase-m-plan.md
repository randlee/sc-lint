# Phase M Plan — atm-core P1 Tool Fixes

Phase M resolves atm-core's P1 issues #786, #720, #198 and #136 by improving
sc-lint's tools and consumer configuration surfaces (ADR-022). The beads are
the source of truth for each sprint's deliverables. This page shows how the
sprints fit together.

## Authority

| Document | Status | Approved by | Gate bead |
| --- | --- | --- | --- |
| [ADR-022](../../sc-lint/adr/ADR-022-tools-and-consumer-configuration.md) tools and consumer configuration | Pending | — | lint-t6i7 blocks dispatch of m-3, m-4 |
| [ADR-010 amendment 2026-10-30](../../sc-lint/adr/ADR-010-portability-scope-and-parity.md) PORT-010 companion shapes | Pending | — | lint-m-adr010 blocks dispatch of m-1 (and so all of stack A) |
| ADR-021 | Withdrawn | — | none; superseded by ADR-022 and the ADR-010 amendment |
| REQ-PRODUCT-004AA (PORT-010), REQ-SCB-019, REQ-CLI-021 | Accepted | — | none; the PORT-010 shape sub-bullets are contingent on the ADR-010 amendment |
| Anchors `v3/anchors.json` ANC-001..013 | Locked; ANC-001/002/003/004/005/008/011/013 rewritten to match the plan, pending review | — | planning and development gates |
| Intent `v3/intent.md` | Rand direction 2026-10-30 | — | framing only |

## Sprints

| Sprint | Stack / layer | Owner | Issue | Anchors |
| --- | --- | --- | --- | --- |
| m-1 port010-same-name-siblings | A1 | clint | #720, #786 S3 | ANC-003, ANC-009 |
| m-5a port010-helper-shapes | A2 | clint | #786 S1, S5 | ANC-001, ANC-005, ANC-009 |
| m-5b port010-renamed-and-rejected-config | A3 | clint | #786 S2, S7 | ANC-002, ANC-008, ANC-009 |
| m-5c port010-portable-sibling-and-unix-module | A4 (top) | clint | #786 S4, S6; #786/#720 closure tests | ANC-004, ANC-006, ANC-007, ANC-009, ANC-010 |
| m-4 port010-site-allow | B1 | cfast | #786 S8 | ANC-010, ANC-009 |
| m-2 manifest004-local-module-paths | C1 | cfast | #136 | ANC-013 |
| m-3 lint-step-env-cwd | D1 | cfast | #198 | ANC-011, ANC-012 |
| m-7 consumer-step-visible-output | E1 | cfast | #195 (visible output) | none |
| m-8 boundary-cycle-finding-paths | F1 | cfast | #385 (path-less cycle findings) | none |

## Branch Stacks And Parallelism

```text
develop
├── A (clint): m-1 ─► m-5a ─► m-5b ─► m-5c ◄── develop merged in after B lands
├── B (cfast): m-4
├── C (cfast): m-2
├── D (cfast): m-3
├── E (cfast): m-7
└── F (cfast): m-8
```

Stack A has four layers. Every layer edits `parity.rs`, `companion_index.rs`
and the port010 fixtures, and each shape builds on the companion index of the
layer below, so splitting A would create parallel edits to the same files.

No owner works two stacks at once: clint owns A; cfast takes C, then B, then
D, E and F. B and D also wait for lint-t6i7.

Shared files between stacks:

| File | Stacks | Owner of the merge |
| --- | --- | --- |
| `crates/sc-lint-portability/src/portability/parity.rs` | A (all layers), B (one `site_allow::allowance_for` call) | m-5c (after B lands) |
| `crates/sc-lint-portability/src/lib.rs` | A (m-5a module list), B (module list, error variants) | m-5c (after B lands) |
| `crates/sc-lint-portability/Cargo.toml` | B (directives dependency) | m-5c (after B lands) |
| `crates/sc-lint-portability/src/tests.rs` | A, B | m-5c (after B lands) |

### Stack protocol

1. **Create.** The lead cuts the bottom layer's worktree from `develop` and
   each higher layer from the pushed head of the layer below.
2. **Link.** After the first push the lead opens the PR (bottom base
   `develop`, higher bases the layer below) and links the stack with one
   `gh stack link --base develop <bottom-pr> ... <top-pr>`, or appends with
   `gh stack link <stack#> <pr>`.
3. **Merge forward.** A lower layer's later commits reach the layers above by
   merge, never by `gh stack sync` or `gh stack rebase`. Once stack B lands on
   develop, the stack writer merges develop into m-5c; this is a stack step,
   not a sprint.
4. **Land.** Stacks B, C and D land independently with
   `gh stack merge <stack#> --yes --merge`. Stack A lands after B, carrying
   develop merged into m-5c.

| Sprint | Stack | Runs in parallel with | Waits for | Unblocked when |
| --- | --- | --- | --- | --- |
| m-1 | A | m-2, m-4, m-3 | lint-phase-m-plan-qa, lint-m-adr010 | gates close |
| m-5a | A | m-2, m-4, m-3 | m-1 unblock milestone | ScopeKey, ItemKind, per-package fixtures and atm_core_inventory.toml committed on sprint/m-1-port010-same-name-siblings |
| m-5b | A | m-4, m-3 | m-5a unblock milestone | companion_index.rs with CompanionIndex::build, production_callers_of, gate_of, has_companion committed on sprint/m-5a-port010-helper-shapes |
| m-5c | A | m-4, m-3 | m-5b unblock milestone; m-4 landed before its closure tests | callers_by_branch and the S2 arm of has_companion committed on sprint/m-5b-port010-renamed-and-rejected-config |
| m-2 | C | A | lint-phase-m-plan-qa | gate closes |
| m-4 | B | A | lint-phase-m-plan-qa, lint-t6i7, m-2 complete (owner) | gates close |
| m-3 | D | A | lint-phase-m-plan-qa, lint-t6i7, m-4 complete (owner) | gates close |
| m-7 | E | A | lint-phase-m-plan-qa | gate closes |
| m-8 | F | A | lint-phase-m-plan-qa | gate closes |

CI and QA run alongside the next layer; they do not delay its start.

Unblock milestones for issues: #136 closes when stack C lands; #198 when
stack D lands; #786 and #720 when stack A lands after B. Each then closes
through its closure bead below, and atm-core removes its 18 PORT-010
exclusions after the release.

## Issue Closure

Phase M is not complete until every closure bead is closed (lint-phase-m
depends on lint-8h3t). Every open issue labelled 0.8.3 or earlier closes only with quality-mgr
evidence posted on the issue (epic lint-8h3t, one closure bead per issue).
The checks were agreed with atm-core on 2026-10-31 (lint-419k).

| Issue | Closure check | Waits for |
| --- | --- | --- |
| #786, #720 | 0 PORT-010 findings over all 18 atm-core paths at its current head, exclusions removed | m-5c |
| #136 | 0 SCB-MANIFEST-004 findings at atm-core head (incl. `commands/mod.rs`), non-owner mutation still reported | m-2 |
| #198 | lint step `env` `PYO3_PYTHON` makes clippy pass with PATH python < 3.11, via bare `sc-lint` | m-3 |
| #643, #376, #192, #586 | the issue's repro; for #643/#376 also atm-core 6ffaa97be clean = 0 and injected faults reported | closed 2026-10-31 with evidence |
| #195 | both streams printed for a failed step; `<log-root>/<step>.log` for every step | m-7 |
| #385 | `exclude_paths_by_rule` removes an SCB-CYCLE-001 finding in one file only | m-8 |
| #777, #778, #782 | 0.8.3 release assets; #782 also needs Rand's go for winget-pkgs | 0.8.3 release (lint-8h3t.1) |

Stack A also checks `port010_inventory_burndown` in every layer, so a shape
that leaves its own atm-core findings reported fails in that layer.

## Determinism

Every gating test uses committed fixtures: no sleeps, retries, network, clock
or host-dependent paths. Sprints test atm-core code only through committed
fixture replays. Runs against atm-core itself belong to the closure beads,
which quality-mgr runs after the sprint lands, not to sprint acceptance.

## Pending Decisions For Rand

1. Accept ADR-022 (lint-t6i7).
2. Accept the ADR-010 amendment (lint-m-adr010), including the `any(unix,
   test)` gate forms and test-reference rule added after review.
3. S6 scope: do `pub` modules and items inside a `#[cfg(unix)] mod x;` count,
   or only non-public ones (ANC-006/007 as locked say all)?
4. Review the anchor rewrites. On Rand's direction "rewrite to match the
   plan", each statement now describes the behaviour the plan builds. Quotes
   are unchanged. The plan questions were tuned again against a frozen oracle,
   and plan gate r11 passes all 8 beads with every anchor covered.
   - ANC-001 v4: the S1 helper rule covers any caller with a companion PORT-010
     recognises (a sibling, a renamed counterpart, a portable alternative, or
     another such helper). New fixtures, including the helper of an S4 pair.
   - ANC-002: renamed pairs need the same return type, and each function is
     called only from the matching gated branch of one caller.
   - ANC-003 v4: siblings must have the same name and kind and be in the same
     scope (a module, an inherent impl or a crate root, including `main.rs`
     and `src/bin`).
   - ANC-004 v4: a Unix transport function is not reported when its caller's
     non-Unix branch or match arm uses an un-gated loopback TCP alternative.
     This replaces the match-arm wording.
   - ANC-005: helpers of any visibility, called only inside `#[cfg(unix)]`
     blocks of a function that also has a `#[cfg(not(unix))]` block.
   - ANC-008: the portable fallback takes the config type and returns an error
     on every path.
   - ANC-011: the pass fixture uses an absolute `PYO3_PYTHON`.
   - ANC-013: `self::`, `crate::` and `super::` paths anywhere in the code, and
     bare `X::` in a module declaring its own `mod X`.
5. The ANC-003 tuner left one case open: a bead that states the outcome for
   only some of the module / inherent impl / crate-root scopes. Recommendation:
   add a fail fixture for that case.
