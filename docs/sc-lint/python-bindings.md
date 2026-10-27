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
| `sc_lint.tests` | the tests shipped inside the wheel (`python -m sc_lint.run_pytests`); the runner discovers its bundled `sc_lint.tests` package and does not assume the consumer checkout has the sc-lint repository's test tree. |

The pyo3 layer is a packaging bridge only: `run` accepts argv tokens, never
shell text, and the package adds no plugin registry or selection policy. The
boundary record is `boundaries/sc-lint-py/python-bindings.toml`.

## How consumers get it

`python .sc-lint/bootstrap.py setup --config sc-lint.toml` provisions
`.sc-lint/venv` next to the config file
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

Consumer tool provisioning reuses the persisted `.sc-lint/install.json`
`tools` object. After ordinary setup has ensured the product and managed wheel,
`sc_lint.setup_tools` checks only the cargo package versions and rustup targets
declared by that consumer; missing entries are installed/added and verified.
Cargo versions are exact stable versions. The product supplies no default tool
pins, and lint execution never installs tools. Setup check/dry-run can inspect
declared tools but never installs or writes. This does not add a new Python
interpreter-selection policy.

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

## Version-sync tables

The released Python helper is invoked as `python -m sc_lint.check_version_sync`.
It accepts `--root <path>` to select the repository root, `--config <path>` to
select an explicit config file, and `--json` for the `sc-lint-python-v1`
adapter envelope. Text mode prints a success summary on a clean run and one
finding per line on stderr when versions differ. Exit codes are `0` for a clean
run (including skipped checks), `1` for findings or an unexpected error, and
`3` for a configuration error.

The success envelope's `data` has exactly `ok`, `findings`, `skipped`, and
`workspace_version`. `data.ok` is false when findings exist while the envelope
`ok` remains true because the check ran successfully. Each finding has exactly
`check`, `file`, `expected`, and `actual`:

| Check | Meaning |
| --- | --- |
| `crate_version` | A workspace member has no valid package version declaration. |
| `internal_path_dep` | A path dependency's pinned version differs from its target member. |
| `cargo_lock` | A workspace package is absent from or mismatched in `Cargo.lock`. |
| `python.version` | A listed static Python project version differs from the workspace version. |
| `python.manifest_missing` | A listed Python manifest does not exist. |
| `winget.manifest_glob` | The configured glob matched no Winget manifests. |
| `winget.package_version` | A Winget `PackageVersion` field differs from the workspace version. |
| `winget.manifest_version` | A Winget `ManifestVersion` field differs from the workspace version. |
| `winget.installer_url` | A Winget installer URL encodes a different or unparseable version. |
| `winget.missing_field` | A configured Winget version or installer URL field is absent. |
| `release_wiring.file_missing` | The configured release workflow does not exist. |
| `release_wiring.fragment` | A configured release workflow fragment is absent. |

Configuration is resolved per `[version_sync]` table: `--config` first, then
`sc-lint.toml`, then `.just/lint-config.toml`; a candidate without that table
does not stop the search. A missing table in every candidate is the structured
`config.table_missing` error. Other malformed or unreadable configuration and
workspace files are configuration errors without a stable code.

`[version_sync.python]` accepts:

| Key | Type and default | Behavior |
| --- | --- | --- |
| `cargo_lock` | Boolean, default `true` | Checks workspace package versions in `Cargo.lock`; `false` disables the check. A missing lockfile is recorded in `data.skipped` as `{check: "cargo_lock", file: "Cargo.lock", reason: "missing"}`. |
| `python_manifests` | Array of repo-relative paths, default `[]` | Each listed `pyproject.toml` must have a static `[project].version` equal to the workspace version. Dynamic, absent, or non-string versions are reported as `python.version`; a missing file is `python.manifest_missing`. |

`[version_sync.winget]` and `[version_sync.release_wiring]` are disabled by
default with `enabled = false`. The Winget table accepts `enabled`,
`manifest_glob` (no default; required when enabled), `package_version_field`
(default `PackageVersion`), `manifest_version_field` (default
`ManifestVersion`), and `installer_url_field` (default `InstallerUrl`). A glob
with no files is a `winget.manifest_glob` finding. The release wiring table
accepts `enabled`, `file` (no default; required when enabled), and
`required_fragments` (default `[]`); absent files and fragments produce
`release_wiring.file_missing` and `release_wiring.fragment` findings. Paths
and globs must be relative to the repository root and stay inside it.

## Release channel

- `release/publish-artifacts.toml` declares the distribution under
  `[[python_distributions]]`; `release/publish-channel-contracts.toml` is the
  source of truth for the PyPI environments and credential names. The release
  workflows resolve channel settings with
  `.github/scripts/release_artifacts.py channel-config --manifest
  release/publish-artifacts.toml --channel pypi`.
- `release.yml` synchronizes Python package metadata from the workspace
  version, validates that version against the release version, builds wheels
  and an sdist, and attaches them to the GitHub Release. It does not install a
  built wheel to inspect `sc_lint.__version__`.
- `release.yml` job `publish-testpypi` uploads configured distributions when
  `target=testpypi` (environment `testpypi`, secret
  `TEST_PYPI_API_TOKEN`).
- `pypi-publish.yml` job `publish` uploads assets of an already published
  release (environment `pypi`, secret `PYPI_API_TOKEN`); rerun it to retry an
  upload.
