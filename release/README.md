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

Release preflight publishes only a commit whose prerelease passed the native
matrix. Its `native_matrix_gate` step runs
`.github/scripts/release_native_gate.py`, which:

- resolves `prerelease/vX.Y.Z` to its commit V;
- requires a successful `Release Native Verify` run titled with that tag and V
  in which every manifest target's `Native <target>` job succeeded (a skipped
  or missing job fails);
- fails if any build input (crates, bindings, docs-bundle, Cargo and toolchain
  files, the release manifest, release build scripts, actions and build
  workflows) changed between V and the release candidate commit C.

A `Prerelease Archive` run triggers the verification on its own. To re-verify
a prerelease, dispatch `Release Native Verify` with the tag and the full commit
SHA the tag names. Each run uploads a `native-verify-report` artifact rendered
with `sc-compose` from the templates in `release/native-verify/`.
