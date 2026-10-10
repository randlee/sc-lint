# Publishing the sc-lint wheel to PyPI

The `sc-lint` Python distribution (`bindings/sc-lint-py`, abi3 wheel for
CPython >= 3.9 plus an sdist) is published by the existing release pipeline.
The adoption kit bootstrap installs `sc-lint==<version>` from PyPI, so 0.6.0
cannot be adopted until the wheel is uploaded. No secret lives in this
repository; the maintainer provisions them in GitHub.

## Pipeline

| Stage | Where | Notes |
|---|---|---|
| Rehearsal | `release-preflight.yml` job `testpypi-rehearsal` | Builds the Linux wheel and sdist, uploads to TestPyPI with `--skip-existing`. Environment `testpypi`, secret `TEST_PYPI_TOKEN`. |
| Build | `release.yml` job `build-python-wheels` | Wheels for linux x86_64, macOS x86_64 and arm64, Windows x86_64 (sdist from linux). Each job installs the built wheel and checks `sc_lint.__version__` equals the release version. |
| Upload | `release.yml` job `publish-pypi` | Needs `release`, which needs `publish-crates`, so PyPI follows crates.io and the GitHub Release. Environment `pypi`, secret `PYPI_TOKEN`; `maturin upload --skip-existing`. |
| Recovery | `pypi-publish.yml` (`workflow_dispatch`) | Downloads the wheels and sdist attached to an already published GitHub Release and uploads them. Never rebuilds or re-tags. |

## Linux compatibility floor

The Linux wheel is built with `maturin --compatibility manylinux2014 --zig`
in both `release.yml` and `release-preflight.yml`, so it is tagged
`manylinux_2_17_x86_64.manylinux2014_x86_64` and needs glibc 2.17 or newer
(Ubuntu 20.04+, Debian 11+, RHEL/Rocky 8+). Building on the bare
`ubuntu-latest` image would tag the wheel with that image's glibc
(`manylinux_2_39`) and make pip fall back to the sdist on older hosts. The
macOS and Windows wheels use maturin's defaults.

The distribution list comes from `[[python_distributions]]` in
`release/publish-artifacts.toml`.

## Required GitHub configuration

Create two environments (Settings > Environments):

- `testpypi` with secret `TEST_PYPI_TOKEN` (a TestPyPI API token).
- `pypi` with secret `PYPI_TOKEN` (a PyPI API token). Add required reviewers
  to this environment so a production upload needs approval.

Do not store tokens in repository secrets or files.

### Trusted publishing (alternative)

PyPI trusted publishing (OIDC) can replace the tokens. It needs a workflow
change that is not made here: grant `id-token: write` to the upload job,
drop `MATURIN_PYPI_TOKEN`, and register the publisher on PyPI/TestPyPI for
repository `randlee/sc-lint`, workflow `release.yml` (and `pypi-publish.yml`
for recovery) and the matching environment name. A new project name can be
registered as a *pending publisher* before the first upload.

## First publish of the `sc-lint` name

On 2026-10-10 the name `sc-lint` returned 404 on both pypi.org and
test.pypi.org, so it is unclaimed. The first successful upload claims it for
the uploading account.

1. Create the TestPyPI and PyPI accounts' tokens. A project-scoped token
   cannot exist before the project does, so the first tokens must be
   account-scoped. After the first upload, replace each with a token scoped to
   the `sc-lint` project and revoke the account-scoped one.
2. Add the environments and secrets above.
3. Run `Release Preflight` via `workflow_dispatch`; confirm
   `testpypi-rehearsal` uploads and `pip install --index-url
   https://test.pypi.org/simple/ --extra-index-url https://pypi.org/simple/
   sc-lint==0.6.0` works in a clean venv (the extra index supplies
   `codespell`).
4. Run the production `Release` workflow for `0.6.0`. After `publish-pypi`
   succeeds, verify `pip install sc-lint==0.6.0` in a clean venv and that
   `python -c "import sc_lint; print(sc_lint.__version__)"` prints `0.6.0`.
5. PyPI files are immutable: a bad upload needs a new version, not a re-upload.

## Re-running a failed upload

Run `pypi-publish.yml` from the Actions tab with `tag` (for example
`v0.6.0`) and `target` (`testpypi` or `production`). It reuses the assets on
that GitHub Release and `--skip-existing` makes it safe to repeat.

## Local checks before a release

```bash
maturin build --release -m bindings/sc-lint-py/Cargo.toml --out dist
maturin sdist -m bindings/sc-lint-py/Cargo.toml --out dist
python3 -m pip install twine && twine check dist/*
python3 -m venv /tmp/clean && /tmp/clean/bin/pip install --find-links dist sc-lint==0.6.0
```

To exercise the adoption bootstrap offline, put the wheel and its
`codespell` dependency in one directory and run
`SC_LINT_WHEEL_DIR=<dir> .sc-lint/bootstrap test --config sc-lint.toml`; it
provisions `.sc-lint/venv` with `--no-index`. `bootstrap setup` also fetches
the GitHub Release archive for the same version, so it only works after that
release exists.
