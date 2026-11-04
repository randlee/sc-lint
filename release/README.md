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
