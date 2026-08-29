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
3. **The `just` interface is exactly ADR-012's four recipes**, each one line
   delegating to `.sc-lint/bootstrap`; a consumer `Justfile` never encodes
   tool knowledge.
4. A profile entry in `sc-lint.toml` may reference only a shipped binary or
   a `sc_lint` module; never a repository-relative script.

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

## Consequences

- Helper and binary versions can never diverge: both derive from one pin.
- The release archive is self-contained; `materialize_*_runtime.py`-style
  scripts in consumers are deleted.
- The wheel matrix (Linux/macOS/Windows × supported CPython) joins the
  release gate; a wheel publish failure blocks the release.
- Rejected alternatives: keep copying `.just/*.py` from the archive (version
  drift, sc-lint#84 class of bugs); write helpers in Rust as subcommands
  (moves configuration policy into a release cycle, contradicts decision 1).
