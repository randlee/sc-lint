---
id: G.3a
title: sc-lint Python Bindings Via Maturin
status: planned
branch: sprint/G.3a-python-bindings
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.3a-python-bindings
stack: B
stack_base: develop
target: develop (via stack B, PR base develop)
owner: cfast
---

# Sprint G.3a — sc-lint Python Bindings Via Maturin

## Goal

- publish a `sc-lint` Python wheel, built with maturin, that carries every
  helper consumers' `just` recipes need, so nothing is copied from the source
  tree

## Hard Dependencies

- none from Stack A; runs in parallel with G.0–G.2 (Stack B, base `develop`)
- reference: `../sc-publish/plugins/sc-publish/.github/scripts/bootstrap_sc_compose.py`
  and `../sc-compose/bindings/` (existing maturin layout in the ecosystem)
- `sc-publish` PyPI channel (`pypi-publish.yml`) already vendored here
- existing helpers: `.just/run_lint.py`, `.just/lint_common.py`,
  `.just/check_version_sync.py`, `.just/python_adapter.py`, `.just/view_*.py`

## Exact Targets

- `bindings/sc-lint-py/Cargo.toml` (new; `cdylib`, `pyo3`, `maturin` backend)
- `bindings/sc-lint-py/pyproject.toml` (new)
- `bindings/sc-lint-py/src/lib.rs` (new; thin pyo3 surface over existing crate APIs — **no new logic**)
- `bindings/sc-lint-py/python/sc_lint/__init__.py` (new)
- `bindings/sc-lint-py/python/sc_lint/{run_lint,lint_common,check_version_sync,view}.py` (moved from `.just/`)
- `.just/*.py` (source-maintainer copies replaced by imports from `sc_lint`)
- `Justfile` (source recipes use the wheel from a local venv)
- `.sc-lint/bootstrap`, `.sc-lint/bootstrap.ps1` (`setup` provisions
  `sc-lint==<minimum_version>` into `.sc-lint/venv`; `upgrade` re-pins)
- `release/publish-artifacts.toml` (add the Python distribution)
- `.github/workflows/release.yml` (wheel matrix: Linux x86_64, macOS x86_64/arm64, Windows x86_64; abi3)
- `docs/sc-lint/python-bindings.md` (new)
- `Cargo.toml` (workspace member)

## Deliverables

- `pip install sc-lint==X.Y.Z` provides module `sc_lint` with the moved
  helpers and a `sc_lint.binary_path()` resolver; wheel version equals the
  workspace version.
- The pyo3 surface exposes only functions the helpers already call on the CLI
  (`version_json()`, `run(argv) -> exit code`); lint logic stays in the
  existing crates.
- `.sc-lint/bootstrap setup --config sc-lint.toml` creates `.sc-lint/venv`
  and installs the wheel matching `minimum_version`; `--check` reports the
  installed wheel version; offline path via `SC_LINT_WHEEL_DIR`.
- `.just/` in this repository contains no copies of helper logic; each file is
  ≤ 5 lines delegating to `sc_lint`.
- Wheels published to PyPI/TestPyPI through the existing `sc-publish` channel
  using the `PYPI_TOKEN` / `TEST_PYPI_TOKEN` environments.

## Acceptance Criteria

- `maturin build --release -m bindings/sc-lint-py/Cargo.toml` succeeds on the
  three OSes in CI; `python -c "import sc_lint; print(sc_lint.__version__)"`
  prints the workspace version.
- Fresh temp workspace after `sc-lint init --just`: `just setup` creates
  `.sc-lint/venv` with `sc_lint` importable; `just lint` runs with no `.just/`
  directory present.
- `find .just -name '*.py' -size +1k` returns nothing.
- `cargo test --workspace` green; `bindings/sc-lint-py/src/lib.rs` ≤ 150 lines.

## Required Validation

- `cargo test --workspace`
- TestPyPI publish dry-run via `release-preflight.yml`

## Out Of Scope

- exposing analyzer internals to Python beyond what the helpers already use
- fixing lint defects (G.3b)
