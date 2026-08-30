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
- none from Stack C; the `.just/` helper overlap with G.3c is a merge-forward
  reconciliation named in `phase-G-plan.md`, not a start dependency
- Phase G's ADR-016 design decision as recorded in the phase plan. G.0
  formalizes that ADR independently on Stack A; G.3a has no Stack A branch
  dependency and must not wait for its commit or merge.
- reference: `../sc-publish/plugins/sc-publish/.github/scripts/bootstrap_sc_compose.py`
  and `../sc-compose/bindings/` (existing maturin layout in the ecosystem)
- `sc-publish` PyPI channel (`pypi-publish.yml`) already vendored here
- existing helpers: all 21 `.just/*.py` modules on `develop` (enumerated
  under Exact Targets), including `.just/run_lint.py`, `.just/lint_common.py`,
  `.just/lint_identity_literals.py`, `.just/check_version_sync.py`,
  `.just/python_adapter.py`, `.just/view_common.py`, `.just/view_findings.py`

## Exact Targets

- `bindings/sc-lint-py/Cargo.toml` (new; `cdylib`, `pyo3`, `maturin` backend)
- `bindings/sc-lint-py/pyproject.toml` (new)
- `bindings/sc-lint-py/src/lib.rs` (new; thin pyo3 surface over existing crate APIs — **no new logic**)
- `bindings/sc-lint-py/python/sc_lint/__init__.py` (new)
- `bindings/sc-lint-py/python/sc_lint/` — all 21 `.just/*.py` helpers moved
  1:1, none dropped: `check_version_sync`, `fixture_constants`,
  `lint_boundaries`, `lint_cargo_deny`, `lint_cargo_modules`,
  `lint_cargo_shear`, `lint_codespell`, `lint_common`,
  `lint_identity_literals`, `lint_line_counts`, `lint_manifests`,
  `lint_sc_boundary`, `lint_sc_portability`, `print_help`, `python_adapter`
  (the Python-side adapter protocol; the Rust `python_adapter.rs` below is
  its caller, not a replacement), `run_fmt`, `run_lint`, `run_pytests`,
  `run_version`, `view_common`, `view_findings`; plus `.just/tests/` moved to
  `sc_lint/tests/`. `lint_common`/`lint_identity_literals` carry G.3c's
  parser fix per the cross-stack reconciliation entry.
- `bindings/sc-lint-py/python/sc_lint/{_binary,source_venv}.py` (new; product
  binary lookup and the source-checkout venv provisioner)
- `boundaries/sc-lint-py/python-bindings.toml` (new)
- `.just/*.py` (every source-maintainer helper removed; the Justfile runs
  `python -m sc_lint.<module>` from `.sc-lint/venv` instead). This deletion
  includes G.3c's `.just/lint_common.py`, `.just/lint_identity_literals.py`,
  and `.just/tests/`: G.3a does not edit them in place, it ports G.3c's parser
  fix into the wheel copy per the `phase-G-plan.md` cross-stack reconciliation
  entry, and the second stack to land keeps the deletion on merge-forward.
- `Justfile` (source recipes use the wheel from a local venv)
- `.sc-lint/bootstrap`, `.sc-lint/bootstrap.ps1` (`setup` provisions
  `sc-lint==<minimum_version>` into `.sc-lint/venv`; `upgrade` re-pins)
- `release/publish-artifacts.toml` (add the Python distribution)
- `.github/workflows/release.yml` (wheel matrix: Linux x86_64, macOS x86_64/arm64, Windows x86_64; abi3)
- `docs/sc-lint/python-bindings.md` (new)
- `Cargo.toml` (workspace member)
- `crates/sc-lint/src/{entry,python_adapter,command}.rs` (embedder entry point `sc_lint._native.run`; helpers dispatched as `python -m sc_lint.<module>`; typed `CommandId::python_tool`)

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
- `.just/` in this repository contains no Python helper logic: `.just/*.py`
  is removed outright (any residual file would be ≤ 5 lines delegating to
  `sc_lint`). This covers G.3c's two `.just/` files by deletion, not by
  in-place rewrite — see the cross-stack reconciliation entry.
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
- `find .just -name '*.py' -size +1k` returns nothing (satisfied by removing
  `.just/*.py`; G.3c's files are removed with the rest, their fixed logic
  lives in the wheel copy).
- `cargo test --workspace` green; `bindings/sc-lint-py/src/lib.rs` ≤ 150 lines.

## Required Validation

- `cargo test --workspace`
- TestPyPI publish dry-run via `release-preflight.yml`

## Out Of Scope

- exposing analyzer internals to Python beyond what the helpers already use
- fixing lint defects (G.3c)
