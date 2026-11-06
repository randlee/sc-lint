# ADR-023 — One Install And Upgrade Path, With Documented File Ownership

| Field | Value |
| --- | --- |
| ID | ADR-023 |
| Status | Pending — Rand acceptance required before sprints o-1 and o-2 are dispatched |
| Deciders | Rand |
| Related | ADR-012, ADR-015, ADR-016, ADR-022; REQ-PRODUCT-019, REQ-PRODUCT-020, REQ-PRODUCT-021, REQ-PRODUCT-022, REQ-PRODUCT-024, REQ-PRODUCT-025 (new); #862, #903 |

## Context

Three routes write sc-lint files into a consumer repository, and they follow
different update rules:

| Route | Writes | Update rule today |
| --- | --- | --- |
| `sc-lint init --just` (Rust CLI) | `sc-lint.toml`, `Justfile`, `.sc-lint/bootstrap.py` | refreshes `bootstrap.py` when it is an unedited prior-release copy (digest list); never rewrites the other two |
| adoption kit from the marketplace plugin (`$CLAUDE_PLUGIN_ROOT/install.py`) | the above plus `.sc-lint/justfile`, the CI workflow, the skill, the agent, `README.sc-lint.md` and a full copy of the kit under `plugins/sc-lint/` | writes missing files only (#862); nothing is ever refreshed |
| adoption kit from the vendored copy (`plugins/sc-lint/install.py`) | same as the plugin route | same, and the installer itself is stale |

`.sc-lint/bootstrap.py` exists as three identical copies in this repository
(`crates/sc-lint/assets/`, `packages/sc-lint-adoption/.sc-lint/`, `.sc-lint/`).
The 0.8.6 upgrade failure (#903) is the result: one route leaves a stale
vendored installer while seeding newer kit tests against it.

`install.py` does not install sc-lint. `.sc-lint/bootstrap.py` installs the
verified release archive into the managed location and provisions
`.sc-lint/venv` with the version-matched wheel (REQ-PRODUCT-020, -024).

Rand, 2026-11-05: "we need to consistently update the repo through all
supported install methods. we can not leave stale scripts to support one
method and not the other." / "does install.py install sc-lint? consumers
should be told to remove it." / "We should always install following the same
path and everything needs to work. not overwriting a file that we maybe use is
not a requirements based decision." / "We need to remove the ambiguity on how
to install and upgrade sc-lint. There must be clear documented config files
and what we 'upgrade' and what customers use for config must be clearly
documented."

## Decision

1. **Two steps, one path.**
   - *Install the binary* through any release channel (Homebrew, release
     archive, Scoop, winget, the `randlee/sc-lint@v1` Action). Every channel
     delivers the same verified artifact (REQ-PRODUCT-020/021/022).
   - *Adopt or upgrade the repository* only with `sc-lint init --just`.
     `sc-lint upgrade` upgrades the binary; `just setup` and `just upgrade`
     call these commands and nothing else. The adoption skill gathers the
     consumer's facts into `.sc-lint/install.json` and then runs
     `sc-lint init --just`; it has no installer of its own. The rendering
     the kit's `install.py` performs today moves behind that command (its
     implementation may call the version-matched wheel helper,
     REQ-PRODUCT-024).
2. **Every file sc-lint writes into a repository has exactly one owner,
   listed in the table below.** Ownership is a requirement (REQ-PRODUCT-025),
   not an installer preference.
   - **Consumer-owned files are the only configuration surface.** Written once
     when missing, never read for rewrite, never overwritten.
   - **Product-owned files are not edit points.** Every `sc-lint init --just`
     brings them to the bytes shipped with the running binary. A product-owned
     file whose content matches no shipped release fails with
     `CLI.SC_LINT_INTEGRATION_CONFLICT`, naming the file and the fix (move it
     aside and rerun); it is never silently kept or silently replaced.
   - **Retired files** are reported by `sc-lint init --just --check` and by
     every `init`, with a one-line instruction to delete them. sc-lint never
     deletes consumer-tracked files itself.
3. **One source per product-owned file.** Each product-owned file is generated
   from one source in this repository (for `bootstrap.py`,
   `crates/sc-lint/assets/bootstrap.py`); every other copy is generated and
   checked by a test, never hand-maintained.
4. **No installer code in consumer repositories.** `plugins/sc-lint/` is no
   longer written, and kit tests are never projected (#903).

### File ownership

| Path in the consumer repository | Owner | On `sc-lint init --just` |
| --- | --- | --- |
| `sc-lint.toml` | consumer | written when missing; never rewritten |
| `Justfile` (root) | consumer | written when missing; never rewritten |
| `.sc-lint/install.json` | consumer | read only; written by the adoption skill, never by sc-lint |
| `.github/workflows/sc-lint.yml` | consumer | written when missing; never rewritten |
| `.sc-lint/bootstrap.py` | product | replaced with the shipped bytes |
| `.sc-lint/justfile` | product | replaced with the shipped bytes |
| `.claude/skills/sc-lint-adoption/**` | product | replaced with the shipped bytes |
| `.claude/agents/sc-lint-adopter.md` | product | replaced with the shipped bytes |
| `README.sc-lint.md` | retired | reported for deletion (offline docs come from `sc-lint docs`, REQ-PRODUCT-021) |
| `plugins/sc-lint/**` (vendored kit, including `install.py` and `tests/`) | retired | reported for deletion |
| `.sc-lint/venv/` | product-managed state, untracked | provisioned by `just setup`; never committed |

A path not in this table is never written by sc-lint.

## Consequences

- Every install route produces the same repository state; an upgrade from any
  earlier release converges on the shipped product-owned files.
- Consumers customize behavior only through `sc-lint.toml`, the root
  `Justfile`, `install.json` and their workflow.
- The marketplace plugin keeps its skill and agent; its `install.py` is
  removed. Offline adoption uses the installed binary, not a vendored copy
  (supersedes ADR-015's vendored fallback).
- Retired files linger until the consumer deletes them, but every `init`
  names them.

## Verification

For each route in the Context table, a test starts from a 0.8.4-shaped
repository, runs the 0.8.7 path, and asserts: product-owned files equal the
shipped bytes, consumer-owned files are byte-identical to before, retired
files are reported, and `just setup`, `just lint` and `just test` pass.
