---
sprint: lint-3x2.3
bead: lint-3x2.3
epic: lint-spx
status: in-progress
branch: chore/sc-publish-kit-adoption-layer
worktree: /Users/randlee/github/sc-lint-worktrees/chore/sc-publish-kit-adoption
pr_target: chore/sc-publish-wheel-release
parent: chore/sc-publish-wheel-release @ 0c3c3a7
closure_type: release
adrs: []
requirements: []
---

# lint-3x2.3 — Adopt the sc-publish release kit

Layer 2 of the 0.6.0 stack (above lint-spx.42, PR #208). Installs the
sc-publish kit byte-for-byte via `install.py`, pinned in
`release/sc-publish-pin.toml`, with `release/sc-publish-consumer-input.json` as
the single sc-lint declaration. Requires the kit's generic
`[documentation_bundle]` contract (sc-publish PR #125, lint-spx.43).

## Deliverables

1. Kit installed from the pinned revision (clone in gitignored
   `.sc-publish-kit/`); repeat `--dry-run` exits 0.
2. Consumer input: 8 crates, 4 targets, `documentation_bundle`
   (`docs-bundle` -> `sc-lint-docs`), Homebrew and the `sc-lint` PyPI wheel.
   WinGet and Scoop are not declared (not current channels).
3. `scripts/release_artifacts.py` reduced to what the kit does not cover:
   sc-lint package-guide/link validation of the bundle, `stage-release-archive`
   and `validate-release-archive` for the CI release-smoke job, and the
   Homebrew layout/formula helpers still exercised by the sc_lint tests. It
   now reads `[documentation_bundle]` from the rendered manifest and derives
   package guides from `packages/*` in `required_files` (one source of truth).
   Removed (kit-covered): inventory, preflight/publish-plan listing,
   manifest/order/preflight validation, version-unpublished, Python
   distribution handling.
4. `.github/workflows/action-major-tag.yml` (sc-lint-owned, dispatch-only).

## Behavior map: old workflow -> new home

| Old behavior | New home |
|---|---|
| release.yml `gate-and-tag` | kit release.yml `gate-and-tag` |
| release.yml `prepare-release-inventory` (emit-inventory + schema check) | **dropped** — kit `release-plan` + `release-preflight.yml`; no inventory artifact is emitted (see gaps) |
| release.yml `pre-publish-audit` (cargo package/check per crate) | kit `release-preflight.yml` `preflight` + `crates-publish.yml` |
| release.yml `publish-crates` | `crates-publish.yml` (re-dispatchable per tag) |
| release.yml `build` (archives, docs staged) | kit `build`; docs staged by `stage-documentation-bundle`, archive checked by `validate-release-archive` |
| release.yml `build-python-wheels` | kit `build-python-wheels` + `build-python-sdists` |
| release.yml `release` (GitHub Release, checksums) | kit `release` |
| release.yml `publish-pypi`, pypi-publish.yml `publish`, release-preflight.yml `testpypi-rehearsal` | `pypi-publish.yml` (verify-release, publish) and kit `publish-testpypi` |
| release.yml `publish-action-major-tag` | `action-major-tag.yml` (dispatch with the release tag) |
| release.yml `update-homebrew` | `homebrew-publish.yml` (`update-tap`) |
| release-preflight.yml `preflight` | kit `release-preflight.yml` |

## Known gaps / decisions for review

- Release inventory JSON is no longer produced by the release workflow.
- The legacy standalone `sc-lint-boundary` Homebrew formula is no longer
  synced; the kit renders the single `sc-lint` formula (which installs all
  four binaries).
- The v1 Action tag move is now a manual dispatch after release, like the
  other post-release channels, rather than an automatic job.
- Archive layout: see the audit below (breaking for bootstrap and the setup action).
- Pin currently references sc-publish PR #125 head `df0531e`; re-pin to the
  merged sc-publish commit before the PR is opened.

## Archive-layout audit (breaking change, needs a ruling)

Old sc-lint archives were flat: `sc-lint`, the backends and `sc-lint-docs/` at
the archive root. Kit archives are `sc-lint_<version>_<target>/bin/<binary>`
plus `sc-lint-docs/` under that single top-level directory.

| Consumer | Assumes | Effect with kit archives |
|---|---|---|
| `packages/sc-lint-adoption/.sc-lint/bootstrap` (`bootstrap_install`) | `extract/sc-lint` | fails: "release archive did not contain sc-lint" |
| `.github/actions/setup-sc-lint/action.yml` (unix, windows) | `$install_dir/sc-lint` and sibling backends | fails: missing executable |
| `scripts/release_smoke.py` + CI release-smoke job | flat archive built by the legacy `stage-release-archive` | still passes, but now smoke-tests a layout that is no longer released |
| Homebrew formula (kit `formula.rb.j2`) | kit layout via `binary_path` | correct by construction |
| PyPI wheel | n/a | unaffected |

Recommended fix (not applied; changes the adoption kit's public bootstrap):
make bootstrap and the setup action accept both layouts (`extract/sc-lint` or
`extract/*/bin/sc-lint`, backends beside it), and point the CI smoke job at
a kit-layout archive. Alternative: keep flat archives, which the kit cannot
produce today.
