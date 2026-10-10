# ADR-016 — Python Wheel Runtime And No Rust For Configuration

| Field | Value |
| --- | --- |
| ID | ADR-016 |
| Status | Accepted |
| Date | 2026-08-29 |
| Deciders | user, flint, team-lead |
| Approved by | user (Rand Lee), 2026-08-29 |
| Related | ADR-012, ADR-015 |

## Context

`sc-lint` is a set of independent lint crates combined only at the CLI. Rust
is the wrong tool for repository configuration, templating, and CI wiring:
it is slow to change, couples policy to a release, and invited the
consumer-specific code of Phase F. Meanwhile consumers today copy 24
`.just/*.py` helpers from the source archive, so a consumer's helper set can
silently diverge from its pinned binary. `sc-publish` already solves the
same problem for `sc-compose` by provisioning a pinned wheel.

## Decision

1. **No Rust for configuration.** Installation, templating, repository facts,
   CI wiring, and consumer scaffolding are Python, declarative assets
   (TOML/YAML/Justfile), skills, and prompts. A thin maturin bridge may
   expose existing CLI behavior to Python, but it adds no configuration
   policy or repository logic. A `configure`-style module in any crate is a
   violation.
2. **The `sc-lint` Python wheel is the runtime delivery for every
   consumer-run helper.** It is built with maturin from
   `bindings/sc-lint-py/` (pyo3 cdylib, ≤150 lines of bridge) plus the
   `sc_lint` Python package that absorbs the former `.just/*.py` helpers. It
   is published to PyPI through the `sc-publish` channel and pinned by
   `[tool.sc-lint].minimum_version`. Nothing is copied from a source
   archive.
3. **The four ADR-012 recipe names are the standard kit-owned entry points.**
   `just test [layer]` and `just lint [profile]` pass an optional positional
   argument directly to bootstrap. `sc-lint.toml` declares named
   `[tool.sc-lint.test.<layer>]` step lists and lint profiles; no argument uses
   the declared default, and `just test all` runs every layer in declaration
   order. The managed import block is the only kit-owned Justfile region;
   consumer recipes outside it are untouched. New layers, profiles, and
   analyzers are declarative configuration, never bootstrap, wheel, or Rust
   edits. ADR-012's four recipes are thus exactly four kit-owned names;
   arguments and consumer-owned recipes are permitted.
4. A **kit-rendered** profile or layer entry in `sc-lint.toml` may reference
   only a shipped binary or a `sc_lint` module; never a repository-relative
   script. Consumer-owned entries (steps the consumer adds to a profile or
   layer outside the kit-rendered set) may invoke any command, including
   repository-local scripts; the kit never renders, validates, or drifts on
   them. (Amended 2026-08-29, approved by user (Rand Lee); previously the
   restriction applied to every entry.)

Resolution sequence performed by `.sc-lint/bootstrap <op>`:

```text
.sc-lint/bootstrap setup
  ├─ read sc-lint.toml [tool.sc-lint].minimum_version
  ├─ ensure .sc-lint/venv (python3 -m venv, idempotent)
  ├─ pip install "sc-lint==<minimum_version>"   # maturin wheel from PyPI
  ├─ python -m sc_lint ensure-binary            # verified release archive
  └─ export SC_LINT_BIN, PATH for this invocation

.sc-lint/justfile (imported by the consumer Justfile):
setup:    .sc-lint/bootstrap setup   --config sc-lint.toml
lint:     .sc-lint/bootstrap lint    --config sc-lint.toml
test:     .sc-lint/bootstrap test    --config sc-lint.toml
upgrade:  .sc-lint/bootstrap upgrade --config sc-lint.toml
```

## How To Extend

Use declarative `sc-lint.toml` lint profiles, test layers, per-analyzer tables,
step argv/exclusions, platform conditionals, and per-step/per-layer `env`.
Pre/post work is an ordinary extra layer step, not a hook DSL. The kit pins only
`sc-lint`; other toolchain pins remain consumer-owned. Validate/view/bench/fuzz
dispatchers are consumer-owned Justfile recipes outside the managed import
block. Patterns observed independently in 2+ consumer repositories may be
promoted into the kit under Locked Principle 9. Kit-rendered entries remain
limited to shipped binaries or `sc_lint` modules (Decision 4); consumer-owned
domain-lint steps may reference repository-local scripts and are the primary
extension point for repo-specific enforcement.

## Consequences

- Helper and binary versions can never diverge: both derive from one pin.
- The release archive is self-contained; `materialize_*_runtime.py`-style
  scripts in consumers are deleted.
- The wheel matrix (Linux/macOS/Windows × supported CPython) joins the
  release gate; a wheel publish failure blocks the release.
- Rejected alternatives: keep copying `.just/*.py` from the archive (version
  drift, sc-lint#84 class of bugs); write helpers in Rust as subcommands
  (moves configuration policy into a release cycle, contradicts decision 1).

## Amendment 2026-10-10 — `test` is consumer-owned; analyzer tables

| Field | Value |
| --- | --- |
| Status | Accepted (Rand, 2026-10-10) |
| Source | Rand's ruling quoted in the ADR-012 amendment 2026-10-10; #194; #201 |

1. Replaces "The four ADR-012 recipe names are the standard kit-owned entry
   points" and "ADR-012's four recipes are thus exactly four kit-owned names"
   in Decision 3: the kit owns `setup`, `lint`, `upgrade` and the managed
   import block. The installer adds sc-lint's recommended `test` recipe
   outside the block only when the consumer `Justfile` has none; after that the
   consumer owns it. The rest of Decision 3 stands.
2. Replaces the `.sc-lint/justfile` lines of the resolution sequence with:

   ```text
   .sc-lint/justfile (imported by the consumer Justfile):
   setup:    .sc-lint/bootstrap setup   --config sc-lint.toml
   lint:     .sc-lint/bootstrap lint    --config sc-lint.toml
   upgrade:  .sc-lint/bootstrap upgrade --config sc-lint.toml

   Recommended test recipe (consumer Justfile, outside the block, added only
   when absent):
   test:     .sc-lint/bootstrap test    --config sc-lint.toml
   ```

3. Extends Decision 1: an analyzer reading its own
   `[tool.sc-lint.analyzers.<name>]` table at run time (#201) is analyzer
   behaviour, not installation configuration.
