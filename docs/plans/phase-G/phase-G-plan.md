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
   Phase G violation.
8. Python helpers that consumers run (`.just` recipes, lint runners, version
   sync) ship as a **maturin-built `sc-lint` wheel** pinned by
   `minimum_version`, provisioned by `.sc-lint/bootstrap` — the same
   mechanism `sc-publish` uses for the `sc-compose` wheel. Nothing is copied
   from a source archive.

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
| G.0 | Abandon Phase F, ADR-015 | governance / docs |
| G.1 | Adoption kit: installer, verbatim assets, templates, fixtures | product code (Python + assets) |
| G.2 | Adoption skill, agent prompts, marketplace entry, docs | skill / docs |
| G.3a | `sc-lint` Python bindings via maturin, published wheel, bootstrap provisioning | packaging / PyPI |
| G.3b | Self-contained release: recipes run from wheel + binary only; fix consumer-blocking lint bugs | Rust crates + release |
| G.4 | First-wave consumer PRs + sc-publish delegation | consumer repos |
| G.5 | Ecosystem rollout | consumer repos (skill only) |

Sequence: G.0 → G.1 → G.2 ∥ G.3a → G.3b → G.4 → G.5. G.4 is the qualification gate;
there is no separate proof phase.

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
