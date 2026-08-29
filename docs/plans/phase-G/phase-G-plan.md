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
| G.0 | Abandon Phase F; ADR-015 (adoption kit), ADR-016 (Python wheel runtime, no Rust for configuration, four-recipe just interface) | governance / docs |
| G.1 | Adoption kit: installer, verbatim assets, templates, fixtures | product code (Python + assets) |
| G.2 | Adoption skill, agent prompts, marketplace entry, docs | skill / docs |
| G.3a | `sc-lint` Python bindings via maturin, published wheel, bootstrap provisioning | packaging / PyPI |
| G.3b | Self-contained release: recipes run from wheel + binary only; fix consumer-blocking lint bugs | Rust crates + release |
| G.4a | sc-publish delegation | external consumer repository |
| G.4b | First-wave greenfield qualification (`wyvern`, `atm-core`) | external consumer repositories |
| G.4c | `sc-compose` migration qualification | external consumer repository |
| G.5 | Ecosystem rollout | consumer repos (skill only) |

G.4a–G.4c are the qualification gate; there is no separate proof phase. The
split keeps the sc-publish delegation, low-risk greenfield proof, and the
existing-scaffolding migration independently reviewable and independently
closable.

## Requirements And Decision Traceability

| Authority | Phase G application | Owning sprint(s) |
| --- | --- | --- |
| REQ-PRODUCT-019 | One SemVer floor, product-owned four-recipe consumer interface, idempotent/non-mutating integration | G.1, G.2, G.4b, G.4c |
| REQ-PRODUCT-020 | Verified artifact activation, managed bootstrap, and no source-checkout helper copying | G.1, G.3a, G.3b, G.4b, G.4c |
| REQ-PRODUCT-021 | Offline documentation remains product-owned and discoverable after kit adoption | G.2, G.3b, G.4b, G.4c |
| REQ-PRODUCT-022 | Reusable verified setup Action is the CI surface installed by the kit | G.1, G.4a, G.4b, G.4c |
| REQ-PRODUCT-023 (added by G.0) | Versioned adoption kit installs and checks one reusable, drift-detectable consumer end state | G.1–G.2, G.4a–G.5 |
| REQ-PRODUCT-024 (added by G.0) | Version-matched wheel delivers all consumer-run Python helpers without a source-tree dependency | G.3a–G.3b, G.4b–G.4c |
| ADR-012 | Four public `just` recipes and product-owned bootstrap ownership | G.0–G.5 |
| ADR-015 / ADR-016 (created by G.0) | Kit ownership and no-Rust-configuration/wheel-runtime design | G.1–G.5 |

## Branch Stacks And Parallelism

Work is organised as two `gh stack` stacks (stacks are strictly linear, so
parallelism is between stacks, never inside one). Every branch has its own
worktree under `../sc-lint-worktrees/<branch>`.

```text
develop (trunk)
 ├─ Stack A — adoption kit (owner: clint)
 │   └── feature/phase-G-planning          this plan
 │        └── sprint/G.0-abandon-phase-F   ADR-015, archive F
 │             └── sprint/G.1-adoption-kit
 │                  └── sprint/G.2-adoption-skill
 └─ Stack B — product (owner: cfast)
     └── sprint/G.3a-python-bindings
          └── sprint/G.3b-self-contained-release

External, non-branch delivery closures (not `gh stack` layers):
  G.4a sc-publish delegation  ─┐
  G.4b wyvern + atm-core      ─┼─ after the published G.2/G.3b release
  G.4c sc-compose migration  ─┘  after G.4b qualification merges
  G.5 remaining-repository rollout after G.4a–G.4c and the approved inventory
```

| Sprint | May run alongside | Must wait for | Exact event that unblocks it |
| --- | --- | --- | --- |
| G.0 | G.3a | Phase G planning branch committed | `feature/phase-G-planning` is committed and linked as Stack A's bottom planning layer |
| G.1 | G.3a | G.0 in Stack A | G.0 is committed on `sprint/G.0-abandon-phase-F`; G.1's PR base is that branch |
| G.2 | G.3a / independent G.3b work | G.1 in Stack A | G.1 is committed on `sprint/G.1-adoption-kit`; G.2's PR base is that branch |
| G.3a | G.0–G.2 | none from Stack A | Stack B branch is created from `develop` and committed work may begin immediately |
| G.3b | G.2 and all non-reconciliation Stack A work | G.3a in Stack B; bootstrap-copy closeout also waits for G.1 to land | G.3a is committed on `sprint/G.3a-python-bindings`; before the G.3b release closes, G.1 has merged to `develop` and that `develop` merge-forward is present in G.3b |
| G.4a | G.4b | released kit Action and self-contained release | G.2 and G.3b are merged and the versioned release containing both is published |
| G.4b | G.4a | released adopter skill and self-contained release | G.2 and G.3b are merged and the versioned release containing both is published |
| G.4c | no product work; G.4a may finish independently | G.4b greenfield qualification | both G.4b consumer PRs merge with their required CI and drift checks green |
| G.5 | external consumer PRs may run together | all first-wave closures and approved inventory | G.4a, G.4b, and G.4c merge; product owner records the remaining-repository inventory in the rollout table |

Cross-stack touch point: G.1 vendors `.sc-lint/bootstrap*` verbatim from
`develop`; G.3a modifies those product files. G.3b is the sole reconciliation
layer: it may begin all product-only work once G.3a is committed, but it may
not claim release closure until G.1 has merged to `develop`, that merge is
merged forward into Stack B, and the kit copy is re-synced byte-for-byte. No
Stack B branch is based on a Stack A branch. G.3a's acceptance therefore uses
`sc-lint init --just` on a fresh workspace, not the G.1 fixture.

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
