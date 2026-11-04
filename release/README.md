# Release Notes Policy

Release notes for sc-lint come from two sources:

- `CHANGELOG.md` is the human-written record of every release. Each release
  adds its section there.
- `.github/workflows/release.yml` publishes each GitHub Release with
  `generate_release_notes: true`, so GitHub generates the release body from
  merged pull requests.

The per-version files in this directory are archived records. They stop at
`RELEASE-NOTES-0.7.0.md` and are not updated for 0.8.0 or later. Do not add
new per-version files unless a release needs a long-form note beyond the
changelog section; when one does, start from `RELEASE-NOTES-TEMPLATE.md`.

## Native Verification Gate

Before `publisher` publishes `vX.Y.Z`, a prerelease must pass the native matrix:

1. `Prerelease Archive` builds the manifest-declared archives and cp39-abi3
   wheels for `prerelease/vX.Y.Z` and publishes them as a GitHub prerelease.
2. `Release Native Verify` runs on that run's completion (or by dispatch with
   the tag `prerelease/vX.Y.Z`). Its run title is `Release Native Verify
   prerelease/vX.Y.Z`. Each target downloads the prerelease archive and wheels,
   checks SHA-256 and architecture, and runs the archive binaries and the
   wheel's `sc-lint` under Python 3.9 (3.11 on win_arm64).
3. The verify run must conclude `success`. A skipped or failed run is not a gate.
4. The prerelease commit `V` must be an ancestor of the release-candidate commit
   `C`, and no artifact input may differ between `V` and `C`. Artifact inputs are
   declared in `.github/scripts/release_native_gate.py`: `crates/`, `bindings/`,
   `docs-bundle/`, the root Cargo files, `release/publish-artifacts.toml`, the
   build scripts, and `prerelease-archive.yml`. Docs, `site/`, the CHANGELOG,
   publish and channel workflows, verifier scripts and tests never require a rerun.

`release-preflight.yml` enforces steps 3 and 4. It reports `V`, the verify run
id, and the classified diff, and fails naming the offending paths when `V` is not
an ancestor of `C` or an artifact input changed.

Each verify target writes a JSON result. The verify run's report job renders one
XHTML panel per target through the pinned sc-compose wheel, tiles the panels into
a primary report with an overview of the trigger, and writes it to
`site/reports/release-native-verify/<stamp>-<tag>-run<id>/index.html`. Its run
index is `site/reports/release-native-verify/index.html`. The report job uploads
the rendered directory as the `native-verify-report` artifact. Committing that
directory to `site/` is a separate step, so the report reaches the repository
website only after it lands on `main`. See
`.claude/skills/publishing/ref/release-state-strategy.md`.
