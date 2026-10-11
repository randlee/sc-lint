# Python Bindings (`sc-lint` wheel)

`bindings/sc-lint-py` packages the repository helper scripts and a thin pyo3
surface over the `sc-lint` CLI crate as the PyPI distribution **`sc-lint`**
(import name `sc_lint`). It is the runtime that `just lint` / `just test`
invoke in every consumer; no consumer copies helper scripts any more
([ADR-016](./adr/ADR-016-python-wheel-runtime-and-no-rust-configuration.md)).

## What the wheel contains

| Surface | Contents |
| --- | --- |
| `sc_lint._native` (pyo3, abi3-py39) | exactly two exports: `version_json() -> str` (the `sc-lint --json version` payload) and `run(argv: list[str]) -> int` (runs the CLI in-process, returns its exit code). No lint logic lives here. |
| `sc_lint.__version__` | parsed from `version_json()`; always equals the workspace version. |
| `sc_lint.binary_path()` | resolves the native `sc-lint` executable: `SC_LINT_BIN`, then the managed install dir (`SC_LINT_INSTALL_DIR`, `$XDG_DATA_HOME/sc-lint/bin`, `~/.local/share/sc-lint/bin`), then `PATH`. |
| `sc_lint.<helper>` modules | the former `.just/*.py` helpers (`run_lint`, `run_pytests`, `lint_*`, `check_version_sync`, `view_findings`, …) run as `python -m sc_lint.<helper>`. |
| `sc_lint.tests` | the helper unit tests (`python -m sc_lint.run_pytests`). |

The pyo3 layer is a packaging bridge only: `run` accepts argv tokens, never
shell text, and the package adds no plugin registry or selection policy. The
boundary record is `boundaries/sc-lint-py/python-bindings.toml`.

## How consumers get it

`.sc-lint/bootstrap setup --config sc-lint.toml` (POSIX) or
`bootstrap.ps1` (Windows) provisions `.sc-lint/venv` next to the config file
and installs `sc-lint==<minimum_version>` into it:

- `setup --check` exits 4 (`CLI.SC_LINT_PYTHON_UNAVAILABLE`) when the venv is
  missing or older than `minimum_version`; `--dry-run` prints what it would
  install.
- `SC_LINT_WHEEL_DIR=<dir>` installs offline with
  `pip install --no-index --find-links <dir>`; the directory must also hold
  the wheel's dependencies (currently `codespell`).
- `lint`, `test`, and `upgrade` provision the venv on demand, so a fresh
  checkout only needs `just lint`.

The Rust CLI runs helper steps as `<venv python> -m sc_lint.<module>`. In this
source repository (no consumer venv yet) it falls back to the host interpreter
with `bindings/sc-lint-py/python` on `PYTHONPATH`, so `cargo test` works
without a venv.

## Source-repository workflow

- `just setup` / `just lint` / `just test` run
  `bindings/sc-lint-py/python/sc_lint/source_venv.py` first: it builds the
  wheel from this checkout into `.sc-lint/wheels/` and installs it into
  `.sc-lint/venv` (rebuilt when the bindings, `crates/sc-lint/src`, or the
  workspace version change). Both directories are git-ignored.
- Fixture tests that exercise the consumer bootstrap point
  `SC_LINT_WHEEL_DIR` at `.sc-lint/wheels`, so they never reach PyPI; they
  skip with a `just setup` hint when the wheels are absent.
- Manual build: `maturin build --release -m bindings/sc-lint-py/Cargo.toml`.

## Release channel

- `release/publish-artifacts.toml` declares the distribution under
  `[[python_distributions]]`; `release/publish-channel-contracts.toml` is the
  source of truth for the PyPI environments and credential names. The release
  workflows resolve channel settings with
  `.github/scripts/release_artifacts.py channel-config --manifest
  release/publish-artifacts.toml --channel pypi`.
- `release.yml` jobs `build-python-wheels` and `build-python-sdists` build the
  declared distributions; wheel jobs verify `sc_lint.__version__` and attach
  their wheels to the GitHub Release.
- `release.yml` job `publish-testpypi` uploads configured distributions when
  `target=testpypi` (environment `testpypi`, secret
  `TEST_PYPI_API_TOKEN`).
- `pypi-publish.yml` job `publish` uploads assets of an already published
  release (environment `pypi`, secret `PYPI_API_TOKEN`); rerun it to retry an
  upload.
