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

Before publishing `vX.Y.Z`, `Prerelease Archive` builds and publishes the
manifest-declared archives and wheels for `prerelease/vX.Y.Z`. `Release Native
Verify` checks each archive and wheel on its native host. Each target writes a
JSON result; the pinned sc-compose wheel renders the report. The report remains a
workflow artifact until `lint-085.77` automates committing it to `site/`.

Preflight enforces two rules:

- **Trusted run.** Only `release-native-verify.yml` can prove the prerelease.
  A `workflow_run` must come from the prerelease tag push; a
  `workflow_dispatch` must come from the default branch. The exact trigger rule
  is defined by [`trusted_verify_run`](../.github/scripts/release_native_gate.py).
- **Verified commit and inputs.** The tag is resolved through `refs/tags` (peeling
  annotated tags), and the manifest and native matrix are read at that commit
  `V`; its verify-run title binds the tag to `V`. A prerelease workflow run
  proves that tag commit, and a dispatch must name it and have a successful
  prerelease build at that commit. `V` must be an ancestor of candidate `C`, and
  no artifact input may differ between them. Artifact inputs and build manifest sections are defined in
  [`release_native_gate.py`](../.github/scripts/release_native_gate.py).

The gate requires every native job and report job to succeed; a successful run
with skipped native jobs does not count. Renames are checked on both paths.
Docs, `site/`, the CHANGELOG, and publish-workflow changes do not require a
rerun. Preflight prints `V`, the verify run id, the changed artifact inputs and
the count of other changed paths; failures name the reason.

The report is written to
`site/reports/release-native-verify/<stamp>-<tag>-run<id>/index.html`, with a
run index at `site/reports/release-native-verify/index.html`. The workflow
uploads it as the `native-verify-report` artifact. See
`.claude/skills/publishing/ref/release-state-strategy.md`.
