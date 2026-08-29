---
id: G.3a
title: sc-lint Python Bindings Via Maturin
status: planned
branch: sprint/G.3a-python-bindings
worktree: /Users/randlee/Documents/github/sc-lint-worktrees/sprint/G.3a-python-bindings
stack: B
stack_base: develop
target: develop (via stack B, PR base develop)
owner: flint
# Owner assignment: clint owns most sprints; cfast takes easy closure/fix work; flint takes the harder parallel Stack B track.
---

# Sprint G.3a — sc-lint Python Bindings Via Maturin

## Goal

- publish a `sc-lint` Python wheel, built with maturin, that carries every
  helper consumers' `just` recipes need, so nothing is copied from the source
  tree

## Hard Dependencies

- none from Stack A; runs in parallel with G.0–G.2 (Stack B, base `develop`)
- Phase G's ADR-016 design decision as recorded in the phase plan. G.0
  formalizes that ADR independently on Stack A; G.3a has no Stack A branch
  dependency and must not wait for its commit or merge.
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
- `boundaries/sc-lint-py/python-bindings.toml` (new)
- `.just/*.py` (source-maintainer copies replaced by imports from `sc_lint`)
- `Justfile` (source recipes use the wheel from a local venv)
- `.sc-lint/bootstrap`, `.sc-lint/bootstrap.ps1` (`setup` provisions
  `sc-lint==<minimum_version>` into `.sc-lint/venv`; `upgrade` re-pins)
- `release/publish-artifacts.toml` (add the Python distribution)
- `.github/workflows/release.yml` (wheel matrix: Linux x86_64, macOS x86_64/arm64, Windows x86_64; abi3)
- `docs/sc-lint/python-bindings.md` (new)
- `Cargo.toml` (workspace member)

## Binding Boundary

The pyo3 layer is a packaging bridge, not a configuration engine. Its public
shape is limited to the existing CLI behavior:

```rust
#[pyfunction]
fn version_json() -> PyResult<String>;

#[pyfunction]
fn run(argv: Vec<String>) -> PyResult<i32>;
```

```python
def binary_path() -> str: ...
```

`run` accepts argv tokens only; it must not accept shell text, inspect a
consumer repository, render templates, or write configuration. This realizes
REQ-PRODUCT-020 and REQ-PRODUCT-024 while retaining ADR-016's no-Rust-
configuration boundary.

## Unblock Milestone

Commit the wheel contract G.3b consumes: `bindings/sc-lint-py/Cargo.toml`,
`pyproject.toml`, and `src/lib.rs` define a buildable maturin package whose
only pyo3 exports are `version_json()` and `run(argv)`; the Python package
exports `binary_path()` and a locally built wheel imports successfully. Report
that commit immediately; G.3b starts from it on
`sprint/G.3a-python-bindings` while G.3a completes multi-platform publication,
helper migration, bootstrap provisioning, CI, and review.

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
- `boundaries/sc-lint-py/python-bindings.toml` (new): structured boundary
  record for the `bindings/sc-lint-py` workspace member — `allowed_dependencies`
  limited to `sc-lint` crate APIs it binds, `forbidden_edges` to every analyzer
  crate, `state = "concrete_landed"`; `docs/architecture.md` crate list already
  names it.

## Acceptance Criteria

- `maturin build --release -m bindings/sc-lint-py/Cargo.toml` succeeds on the
  three OSes in CI; `python -c "import sc_lint; print(sc_lint.__version__)"`
  prints the workspace version.
- The `sc_lint` package dispatches optional test-layer and lint-profile
  positional arguments to bootstrap only; it adds no plugin registry or
  selection policy.
- Fresh temp workspace after the product generator `sc-lint init --just`
  (retained per the REQ-PRODUCT-019 supersession note; consumers install via
  the G.1 kit): `just setup` creates
  `.sc-lint/venv` with `sc_lint` importable; `just lint` runs with no `.just/`
  directory present.
- `find .just -name '*.py' -size +1k` returns nothing.
- `cargo test --workspace` green; `bindings/sc-lint-py/src/lib.rs` ≤ 150 lines.

## Required Validation

- `cargo test --workspace`
- TestPyPI publish dry-run via `release-preflight.yml`

## Out Of Scope

- exposing analyzer internals to Python beyond what the helpers already use
- fixing lint defects (G.3c)
