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
2. `Release Native Verify` runs on that run's completion, or by dispatch with the
   tag `prerelease/vX.Y.Z` and the full commit SHA the tag names. Its run title is
   `Release Native Verify <tag> <commit>`, so one run proves one tag at one commit.
   For a `Release` run the title's first field is the triggering branch; the
   report names the tag. Each target downloads the prerelease archive and wheels,
   checks SHA-256 and architecture, and runs the archive binaries and the wheel's
   `sc-lint` under Python 3.9 (3.11 on win_arm64).
3. The gate accepts a run only when it concluded `success` and every native job
   and the report job concluded `success` for that tag and commit. A run that
   concludes `success` while its native jobs were skipped proves nothing and is
   rejected. A run for an earlier commit of a re-cut tag does not match.
4. The prerelease commit `V` must be an ancestor of the release-candidate commit
   `C`, and no artifact input may differ between `V` and `C`. Renames are checked
   on both their old and new paths. The artifact inputs are `crates/`,
   `bindings/`, `docs-bundle/`, `.github/actions/`, `Cargo.toml`, `Cargo.lock`,
   `rust-toolchain.toml`, `build_release_target.sh`, `prerelease-archive.yml`, the
   release scripts that the archive build runs and their local imports
   (`release_artifacts.py`, `release_manifest.py`, `release_platforms.py`,
   `release_python.py`, `release_registry.py`, `release_stage.py`,
   `verify_release_arch.py`, `wheel_contents.py`), and the manifest's build
   sections (`project`, `release_targets`, `crates`, `release_binaries`,
   `python_distributions`). Docs, `site/`, the CHANGELOG, publish and channel
   workflows, the native verifier and its tests, and the manifest's channel and
   prerelease sections never require a rerun. The list is defined in
   `.github/scripts/release_native_gate.py`, and its tests fail if a script the
   archive build runs is not covered.

`release-preflight.yml` enforces steps 3 and 4. On success it prints `V`, the
verify run id, the artifact-input list (`none` when nothing changed), and how many
paths need no rerun. On failure it names the offending paths or the reason.

Each verify target writes a JSON result. The report job renders one XHTML panel
per target through the pinned sc-compose wheel and tiles the panels into a primary
report. Its overview names the tag, the source workflow, the commit, the trigger
and the verify run. The report is written to
`site/reports/release-native-verify/<stamp>-<tag>-run<id>/index.html`, with a run
index at `site/reports/release-native-verify/index.html`. The report job uploads
the rendered directory as the `native-verify-report` workflow artifact. Committing
it to `site/` is a manual step until `lint-085.77` automates it, so the report
reaches the repository website only after it lands on `main`. See
`.claude/skills/publishing/ref/release-state-strategy.md`.
