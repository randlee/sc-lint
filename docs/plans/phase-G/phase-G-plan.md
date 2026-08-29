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
   repo-specific.** Rust in this repository implements lint analysis and the
   `sc-lint` CLI only. Installation, templating, repo facts, CI wiring, and
   consumer scaffolding are Python, TOML/YAML/Justfile assets, skills, and
   prompts. A sprint that adds a `configure`-style module to any crate is a
   Phase G violation. (ADR-016)
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
| G.4 | First-wave consumer PRs + sc-publish delegation | consumer repos |
| G.5 | Ecosystem rollout | consumer repos (skill only) |

G.4 is the qualification gate; there is no separate proof phase.

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
```

| Runs in parallel | Must be sequential | Why |
| --- | --- | --- |
| Stack A and Stack B, from day one | — | disjoint paths: A touches `packages/`, `docs/`, `.claude-plugin/`; B touches `bindings/`, `crates/`, `.just/`, `.sc-lint/bootstrap*`, release workflows |
| G.1 implementation may start once G.0 is *committed* (not merged) | G.0 → G.1 → G.2 within Stack A | each layer's PR base is the layer below; the reviewer sees only that layer's diff |
| G.3b may start once G.3a is committed | G.3a → G.3b within Stack B | G.3b runs recipes through the G.3a wheel |
| G.4 sc-publish delegation PR, as soon as G.1 is committed | — | it only needs the kit action's name and input contract |
| G.4 `wyvern` and `atm-core` consumer PRs, together | after a `sc-lint` release containing both stacks | greenfield repos, no interaction |
| — | G.4 `sc-compose` after `wyvern`/`atm-core` prove the kit | it is the only consumer with existing scaffolding to remove (D1) |
| G.5 all remaining repositories, together | after G.4 merges | skill-only, no product change permitted |

Cross-stack touch point: G.1 vendors `.sc-lint/bootstrap*` verbatim from
`develop`; G.3a modifies those product files. G.3b (above G.3a, and started
only after Stack A has merged) re-syncs the kit's copy and is the single place
that reconciles the two. G.3a's acceptance therefore uses `sc-lint init --just`
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
6. Sprint docs for G.4 and G.5 have no stack: they are PRs in other
   repositories.

Sprint docs:
- [sprint-G0.md](./sprint-G0.md)
- [sprint-G1.md](./sprint-G1.md)
- [sprint-G2.md](./sprint-G2.md)
- [sprint-G3a.md](./sprint-G3a.md)
- [sprint-G3b.md](./sprint-G3b.md)
- [sprint-G4.md](./sprint-G4.md)
- [sprint-G5.md](./sprint-G5.md)

## Decisions Required Before G.1 Dispatch

| # | Decision | Recommendation |
| --- | --- | --- |
| D1 | Uniform `just lint` vs sc-compose's `sc-compose lint` wrapper | `just lint` calls `.sc-lint/bootstrap lint` directly. `sc-compose lint` keeps only its native `template-contracts` target; the sc-lint-forwarding targets in `.sc/sc-lint/targets/*.toml` are removed in the sc-compose consumer PR (G.4). |
| D2 | Kit location | `packages/sc-lint-adoption/` in this repository (sibling of `packages/sc-lint-version-adoption`), vendored into consumers as `plugins/sc-lint/`. |
| D3 | New-repo template home | Out of scope for Phase G; G.1's empty-repo fixture is built so it can be promoted to the template unchanged. |

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
