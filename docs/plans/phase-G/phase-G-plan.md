---
title: Phase G Plan — Standard Repo Tools: sc-lint Adoption Kit
status: planned
branch: feature/phase-G-planning
target: develop
supersedes: integrate/phase-F (abandoned, unmerged)
---

# Phase G Plan — sc-lint Adoption Kit

## Goal

Every sc-ecosystem Rust repository (~10+, first wave `wyvern`, `atm-core`,
`sc-compose`) adopts the released `sc-lint` product the same way, reaches the
same committed end state, and can prove it is still on the standard with one
drift check. New Rust repositories start from that same end state.

The delivery vehicle is a vendorable **adoption kit** in the exact form of
`sc-publish` (`plugins/sc-publish`): skills + agent prompts + GitHub
actions/workflows + `install.py`, copied byte-for-byte into consumers, with
drift reported as a defect. It is not a Rust engine.

## Locked Principles

1. `sc-lint` crates work against **any** Rust repository. No crate, script,
   template, fixture, or skill in this repository may name or fingerprint a
   specific consumer repository.
2. Anything that would exist in two consumer repositories belongs in a kit,
   never copied into a third.
3. The released `sc-lint` archive is self-contained. A consumer never
   materializes helpers from the source tree (`.just/*.py`, `run_lint.py`, etc.).
4. `sc-lint.toml` `[tool.sc-lint].minimum_version` is the only version pin in
   a consumer. `sc-publish` delegates sc-lint setup to the kit instead of
   carrying its own pin.
5. The consumer interface is ADR-012 unchanged: `just setup | lint | test |
   upgrade` delegating to `.sc-lint/bootstrap`.
6. Greenfield and adoption produce an identical end state: the new-repo
   template **is** the kits applied to an empty Cargo workspace.
7. **No Rust is written for configuration of the tools or for anything
   repo-specific.** Installation, templating, repo facts, CI wiring, and
   consumer scaffolding are Python, TOML/YAML/Justfile assets, skills, and
   prompts. The thin maturin bridge in G.3a may expose existing CLI behavior,
   but it adds no configuration policy or repository logic. A sprint that
   adds a `configure`-style module to any crate is a Phase G violation.
   (ADR-016)
8. Python helpers that consumers run (`.just` recipes, lint runners, version
   sync) ship as a **maturin-built `sc-lint` wheel** pinned by
   `minimum_version`, provisioned by `.sc-lint/bootstrap` — the same
   mechanism `sc-publish` uses for the `sc-compose` wheel. Nothing is copied
   from a source archive. (ADR-016)
9. The kit covers the common base only and documents how to extend; it does
   not try to anticipate consumer-specific needs. Extension points proven
   common across at least two consumers may be promoted from consumer-local
   extension into the kit or wheel as shared base behavior.

## Why Phase F Is Abandoned

Phase F (ADR-014, `integrate/phase-F`, never merged to `develop`) built a
`sc-lint configure` Rust engine (~1.6k LOC), a Python launcher carrying 24
sha256 fingerprints of sc-compose 0.4 files, and a Wyvern wizard. Its target
shape — sc-compose before `sc-publish` — no longer exists, and the files it
was designed to delete are the ones `sc-publish` now installs
(`.github/actions/setup-sc-lint`, `setup-lint-toolchain`). The premise decayed;
this is a replan, not a fix. Nothing from Phase F is on `develop`. Any Phase F
content worth keeping is recovered explicitly in `G.1` (see its "Recovered
From Phase F" list); everything else is discarded.

## ADR Status

| ADR | Title | Status | Approved by |
| --- | --- | --- | --- |
| ADR-015 | Standard Repo Tools Adoption Kit | Accepted | user (Rand Lee) |
| ADR-016 | Python Wheel Runtime And No Rust For Configuration | Accepted (Decision 4 amended 2026-08-29: kit-rendered entries only) | user (Rand Lee) |

The ADRs, requirements, and architecture traceability landed on this planning
branch before implementation. The user accepted both ADRs on 2026-08-29, so no
Phase G orchestration approval remains pending.

## Consumer End State

```text
sc-lint.toml                          # rendered from install.json; sole version pin
.sc-lint/bootstrap, bootstrap.ps1     # verbatim (already product-owned, ADR-012)
.sc-lint/justfile                     # verbatim: setup / lint / test / upgrade
Justfile                              # consumer-owned + one marked import block
.github/actions/setup-sc-lint/        # verbatim; reads sc-lint.toml
.github/workflows/sc-lint.yml         # verbatim
plugins/sc-lint/                      # the vendored kit itself
README.sc-lint.md                     # kit README, renamed on install
```

## Sprint Structure

| Sprint | Title | Closure type |
| --- | --- | --- |
| G.0 | Archive the rejected Phase F line | git housekeeping |
| G.1 | Adoption kit: installer, verbatim assets, templates, fixtures | product code (Python + assets) |
| G.2 | Adoption skill, agent prompts, marketplace entry, docs | skill / docs |
| G.3a | `sc-lint` Python bindings via maturin, published wheel, bootstrap provisioning | packaging / PyPI |
| G.3b | Self-contained release: recipes run from wheel + binary only | Rust crates + release |
| G.3c | Identity-literals unicode-escape parser defect fix | targeted lint fix |
| G.4a | sc-publish delegation | external consumer repository |
| G.4b | `wyvern` greenfield qualification plus `atm-core` migration | external consumer repositories |
| G.4c | `sc-compose` migration qualification | external consumer repository |
| G.5 | Ecosystem rollout | consumer repos (skill only) |

G.4a–G.4c are the qualification gate; there is no separate proof phase. The
split keeps the sc-publish delegation, low-risk greenfield proof, and the
existing-scaffolding migration independently reviewable and independently
closable.

## Requirements And Decision Traceability

| Authority | Phase G application | Owning sprint(s) |
| --- | --- | --- |
| REQ-PRODUCT-019 (as superseded by ADR-015 for the installation mechanism) | One SemVer floor, product-owned four-recipe consumer interface, idempotent/non-mutating kit integration (`--dry-run` drift exit 1, user-owned conflict exit 2) | G.1, G.2, G.4b, G.4c |
| REQ-PRODUCT-020 | Verified artifact activation, managed bootstrap, and no source-checkout helper copying | G.1, G.3a, G.3b, G.4b, G.4c |
| REQ-PRODUCT-021 | Offline documentation remains product-owned and discoverable after kit adoption | G.2, G.3b, G.4b, G.4c |
| REQ-PRODUCT-022 | Reusable verified setup Action is the CI surface installed by the kit | G.1, G.4a, G.4b, G.4c |
| REQ-PRODUCT-023 (planning branch) | Versioned adoption kit installs and checks one reusable, drift-detectable consumer end state | G.1–G.2, G.4a–G.5 |
| REQ-PRODUCT-024 (planning branch) | Version-matched wheel delivers all consumer-run Python helpers without a source-tree dependency | G.3a–G.3b, G.4b–G.4c |
| ADR-012 | Four public `just` recipes and product-owned bootstrap ownership | G.0–G.5 |
| ADR-015 / ADR-016 (planning branch; Accepted before orchestration) | Kit ownership and no-Rust-configuration/wheel-runtime design | G.1–G.5 |

## Branch Stacks And Parallelism

Work is organised as two `gh stack` stacks (stacks are strictly linear, so
parallelism is between stacks, never inside one). Every branch has its own
worktree under `../sc-lint-worktrees/<branch>`.

### Stack Architecture Record

The stack design was derived from deliverables and path sets before the sprint
documents were written:

| Candidate stack | Deliverable/path set | Ordered layers and reason | Owner |
| --- | --- | --- | --- |
| A — adoption kit | Governance and traceability under `docs/`; generic kit, fixtures, and CI under `packages/sc-lint-adoption/`, `tests/adoption/`, and `.github/workflows/ci.yml`; then the adoption skill, marketplace, and consumer guide under `packages/sc-lint-adoption/.claude/`, `.claude-plugin/`, and `docs/sc-lint/` | G.0 archives Phase F; G.1 supplies the generic kit contract; G.2 consumes that contract in agent-facing guidance. These are one coherent kit path set. | cfast (G.0); clint (G.1–G.2) |
| B — product runtime | Wheel binding and bootstrap/release paths under `bindings/sc-lint-py/`, `.sc-lint/`, `.just/`, `crates/`, `scripts/`, and release workflows | G.3a establishes wheel entry points; G.3b closes the self-contained archive. This path set is independent of Stack A until the named reconciliation below. | flint |
| C — targeted parser fix | Rust-literal parser and identity-literals utility under `.just/lint_common.py` and `.just/lint_identity_literals.py`, with focused tests under `.just/tests/` | G.3c is a disjoint, independently closable consumer-blocking defect fix rooted directly on `develop`; validation is the focused Python suite plus the `just lint` identity-literals target. | cfast |
| External qualification | Consumer-only paths in `../sc-publish`, `../wyvern`, `../atm-core`, and `../sc-compose` | G.4a is the independently closable sc-publish delegation; G.4b qualifies greenfield `wyvern` while migrating `atm-core` as one coupled release gate; G.4c consumes wyvern's proven greenfield artifact for the established-workspace migration. G.5 is the separately authorized remaining-repository rollout. | clint (G.4a–G.5) |

The only overlapping product/kit path is `.sc-lint/bootstrap*`: G.1 vendors
the product snapshot and G.3a changes the product implementation. G.3b is
the sole, higher reconciliation layer and only re-syncs the kit copy after
the named Stack A merge. No stack takes another stack's commit as its branch
base. Stack A has three delivery layers and Stack B three, so neither requires
further subdivision.

```text
develop (trunk)
 ├─ Stack A — adoption kit (owner: clint)
 │   └── feature/phase-G-planning          this plan
 │        └── sprint/G.0-abandon-phase-F   ADR-015, archive F
 │             └── sprint/G.1-adoption-kit
 │                  └── sprint/G.2-adoption-skill
 └─ Stack B — product (owner: flint)
     └── sprint/G.3a-python-bindings
          └── sprint/G.3b-self-contained-release
 └─ Stack C — targeted fix (owner: cfast)
     └── sprint/G.3c-identity-literals-unicode-fix  PR base: develop

External, non-branch delivery closures (not `gh stack` layers):
  G.4a sc-publish delegation  ─┐
  G.4b wyvern + atm-core      ─┼─ after the published G.2/G.3b release
  G.4c sc-compose migration  ─┘  after G.4b qualification merges
  G.5 remaining-repository rollout after G.4a–G.4c and the approved inventory
```

| Sprint | Stack | Runs in parallel with | Waits for | Unblocked when |
| --- | --- | --- | --- | --- |
| G.0 | A | G.3a and G.3b (Stack B), and G.3c (Stack C) | Phase G planning layer | `feature/phase-G-planning` is committed as Stack A's bottom planning layer. |
| G.1 | A | G.3a and G.3b (Stack B), and G.3c (Stack C) | G.0 unblock milestone | G.0's unblock milestone is committed on `sprint/G.0-abandon-phase-F`. |
| G.2 | A | G.3a and G.3b (Stack B), and G.3c (Stack C) | G.1 unblock milestone | G.1's unblock milestone is committed on `sprint/G.1-adoption-kit`. |
| G.3a | B | G.0–G.2 (Stack A) and G.3c (Stack C) | No lower sprint; Stack B roots on `develop` | Stack B's bottom layer starts immediately from `develop`; it has no lower-sprint milestone. |
| G.3b | B | G.0–G.2 (Stack A) and G.3c (Stack C) | G.3a unblock milestone | G.3a's unblock milestone is committed on `sprint/G.3a-python-bindings`. |
| G.3c | C | G.0–G.2 and Stack B | No lower sprint; Stack C roots on `develop` | Stack C's bottom layer starts immediately from `develop`; it has no lower-sprint milestone or cross-stack touch point. |
| G.4a | external-non-branch | G.4b | Released kit Action and self-contained release | The versioned `sc-lint` release containing G.2 and G.3b is published. |
| G.4b | external-non-branch | G.4a | Released adopter skill and self-contained release | The versioned `sc-lint` release containing G.2 and G.3b is published. |
| G.4c | external-non-branch | G.4a may finish independently | G.4b greenfield qualification | Both G.4b consumer PR merge commits exist with their required CI and drift checks green. |
| G.5 | external-non-branch | Approved remaining-consumer PRs may run together | G.4a–G.4c closures and approved inventory | The G.4a, G.4b, and G.4c external PR merge commits are recorded and the product owner commits the approved inventory to `docs/sc-lint/adoption.md`. |

Cross-stack touch point: G.1 vendors `.sc-lint/bootstrap*` verbatim from
`develop`; G.3a modifies those product files. G.3b is the sole reconciliation
layer: it starts when G.3a's unblock milestone commits and may implement all
product-only work while Stack A proceeds. It may not claim **release closure**
until G.1 has merged to `develop`, that merge is merged forward into Stack B,
and the kit copy is re-synced byte-for-byte. That later merge is a named
reconciliation condition, not a start dependency. No Stack B branch is based
on a Stack A branch. G.3a's acceptance therefore uses `sc-lint init --just`
on a fresh workspace, not the G.1 fixture.

### Stack protocol

The repository rule is merge-forward, never rebase; `gh stack sync` and
`gh stack rebase` are therefore **not used**. The protocol is:

1. Branches and worktrees are created with plain `git worktree add -b
   <branch> <path> <parent>` (done by team-lead at phase start; see the
   tracking table in `../sc-lint-worktrees/worktree-tracking.md`).
2. The developer commits only in the sprint's worktree. When a lower layer
   changes, the lower branch is merged **forward** into every branch above it
   (`git merge --no-ff <lower>`), never rebased.
3. PRs are opened and chained with the API-only command, from the main
   checkout, bottom to top, once the bottom layer is ready for QA:
   `gh stack link --base develop feature/phase-G-planning sprint/G.0-abandon-phase-F sprint/G.1-adoption-kit sprint/G.2-adoption-skill`
   and `gh stack link --base develop sprint/G.3a-python-bindings sprint/G.3b-self-contained-release`.
   Later layers are appended with `gh stack link <stack-number> <branch>`.
4. QA runs per layer on its PR. Landing is `gh stack merge <pr> --yes --merge`
   (merge commits, up to and including that PR), only after QA PASS and the
   user's explicit approval per layer; never `gh pr merge`.
5. After a layer lands, `develop` is merged forward into the remaining
   worktrees of both stacks.
6. Sprint docs for G.4a–G.5 are explicitly external, non-branch closures:
   each consumer PR follows its target repository's branch policy and may not
   change this repository except through a separately planned release fix.

Sprint docs:
- [sprint-G0.md](./sprint-G0.md)
- [sprint-G1.md](./sprint-G1.md)
- [sprint-G2.md](./sprint-G2.md)
- [sprint-G3a.md](./sprint-G3a.md)
- [sprint-G3b.md](./sprint-G3b.md)
- [sprint-G3c.md](./sprint-G3c.md)
- [sprint-G4a.md](./sprint-G4a.md)
- [sprint-G4b.md](./sprint-G4b.md)
- [sprint-G4c.md](./sprint-G4c.md)
- [sprint-G5.md](./sprint-G5.md)

## Decisions Required Before G.1 Dispatch

| # | Decision | Recommendation |
| --- | --- | --- |
| D1 | Uniform `just lint` vs sc-compose's `sc-compose lint` wrapper | `just lint` calls `.sc-lint/bootstrap lint` directly. `sc-compose lint` keeps only its native `template-contracts` target; the sc-lint-forwarding targets in `.sc/sc-lint/targets/*.toml` are removed in the sc-compose consumer PR (G.4c). |
| D2 | Kit location | `packages/sc-lint-adoption/` in this repository (sibling of `packages/sc-lint-version-adoption`), vendored into consumers as `plugins/sc-lint/`. |
| D3 | New-repo template home | Out of scope for Phase G; G.1's empty-repo fixture is built so it can be promoted to the template unchanged. |
| D4 | Remaining-repository inventory | Before G.5 dispatch, product owner records the exact Rust-repository list and exclusions in `docs/sc-lint/adoption.md`; G.5 does not infer membership from local directories or GitHub search. |

## Exit Criteria

- `integrate/phase-F` archived; no Phase F worktree remains.
- `packages/sc-lint-adoption` installs into an empty workspace and a synthetic
  established workspace with `--dry-run` exit 0 after install, exit 1 on any
  drift, on Linux, macOS, and Windows CI.
- The released `sc-lint` binary plus the published `sc-lint` wheel run every
  kit recipe with no source-tree helper and no copied script.
- `grep -rn "configure" crates/*/src` matches nothing outside lint-rule
  configuration loading (`sc-lint.toml` parsing).
- `wyvern`, `atm-core`, `sc-compose` each have a merged consumer PR whose CI
  runs `just lint` and `just test` through the kit, and `sc-publish` no longer
  carries an sc-lint version pin.
- Zero occurrences of a consumer repository name in `packages/`, `crates/`,
  `scripts/`, or `.claude/skills/sc-lint-adoption/`.
