# Publishing the sc-lint wheel to PyPI

The `sc-lint` Python distribution (`bindings/sc-lint-py`, abi3 wheel for
CPython >= 3.9 plus an sdist) is published by the existing release pipeline.
Version 0.6.0 was cut but never published; 0.7.0 is the first PyPI release.
The adoption kit bootstrap installs `sc-lint==0.7.0` from PyPI, so 0.7.0
cannot be adopted until the wheel is uploaded. No secret lives in this
repository; the maintainer provisions them in GitHub.

## Pipeline

| Stage | Where | Notes |
|---|---|---|
| Rehearsal | `release.yml` job `publish-testpypi` when dispatch input `target` is `testpypi` | Uploads the configured Python distributions to TestPyPI with `--skip-existing`. Environment `testpypi`; the channel contract names the credential `TEST_PYPI_API_TOKEN`. |
| Build | `release.yml` jobs `build-python-wheels` and `build-python-sdists` | Builds the manifest-declared wheels and sdist; wheel jobs verify `sc_lint.__version__` against the release version. |
| Upload | `pypi-publish.yml` workflow (`workflow_dispatch`), job `publish`, with target `production` | Uploads verified wheels and sdist from an already published GitHub Release. Environment `pypi`; the channel contract names the credential `PYPI_API_TOKEN`. The workflow selects maturin or twine from the manifest. |
| Recovery | Re-run `pypi-publish.yml` with the same tag and target | Reuses the immutable GitHub Release assets. It never rebuilds or re-tags. |

## Linux compatibility floor

The Linux wheel is built with `maturin --compatibility manylinux2014 --zig`
in `release.yml`, so it is tagged
`manylinux_2_17_x86_64.manylinux2014_x86_64` and needs glibc 2.17 or newer
(Ubuntu 20.04+, Debian 11+, RHEL/Rocky 8+). Building on the bare
`ubuntu-latest` image would tag the wheel with that image's glibc
(`manylinux_2_39`) and make pip fall back to the sdist on older hosts. The
macOS and Windows wheels use maturin's defaults.

The distribution list comes from `[[python_distributions]]` in
`release/publish-artifacts.toml`. The standard PyPI environments and secret
names come from `release/publish-channel-contracts.toml`.

## Required GitHub configuration

Create two environments (Settings > Environments):

- `testpypi` with secret `TEST_PYPI_API_TOKEN` (a TestPyPI API token).
- `pypi` with secret `PYPI_API_TOKEN` (a PyPI API token). Add required
  reviewers to this environment so a production upload needs approval.

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
3. Run the `Release Preflight` workflow via `workflow_dispatch` for `0.7.0`
   and confirm its preflight checks pass.
4. Run the `Release` workflow via `workflow_dispatch` with `target=testpypi`;
   confirm `publish-testpypi` uploads and
   `pip install --index-url
   https://test.pypi.org/simple/ --extra-index-url https://pypi.org/simple/
   sc-lint==0.7.0` works in a clean venv (the extra index supplies
   `codespell`).
5. Run the `Release` workflow for `0.7.0` with `target=production`.
   After the GitHub Release is published, run `Publish PyPI` via
   `workflow_dispatch` with tag `v0.7.0` and target `production`. Verify
   `pip install sc-lint==0.7.0` in a clean venv and that
   `python -c "import sc_lint; print(sc_lint.__version__)"` prints `0.7.0`.
6. PyPI files are immutable: a bad upload needs a new version, not a re-upload.

## Re-running a failed upload

Run `pypi-publish.yml` from the Actions tab with `tag` (for example
`v0.7.0`) and `target` (`testpypi` or `production`). It reuses the assets on
that GitHub Release and `--skip-existing` makes it safe to repeat.

## Local checks before a release

```bash
maturin build --release -m bindings/sc-lint-py/Cargo.toml --out dist
maturin sdist -m bindings/sc-lint-py/Cargo.toml --out dist
python3 -m pip install twine && twine check dist/*
python3 -m venv /tmp/clean && /tmp/clean/bin/pip install --find-links dist sc-lint==0.7.0
```

To exercise the adoption bootstrap offline, put the wheel and its
`codespell` dependency in one directory and run
`SC_LINT_WHEEL_DIR=<dir> .sc-lint/bootstrap test --config sc-lint.toml`; it
provisions `.sc-lint/venv` with `--no-index`. `bootstrap setup` also fetches
the GitHub Release archive for the same version, so it only works after that
release exists.
