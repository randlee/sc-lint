# Release Tag Protection

This document defines the high-level tag discipline for `sc-lint` releases.

## Rules

- Release tags must be created from the reviewed and merged release line.
- Do not reuse a release tag after a failed release attempt.
- If a release workflow fails after tagging, start a new release cycle with a
  new version rather than mutating the original tag.
- `develop` remains the staging branch; `main` is the protected release branch.

## Operational Expectations

- Release automation must validate that the repo version, crate versions, and
  release artifacts all agree before publication.
- Release notes must exist before `develop -> main` merge for a release.
- Any release blocker should be surfaced immediately to `team-lead`.

## Related Docs

- [release/RELEASE-NOTES-TEMPLATE.md](../release/RELEASE-NOTES-TEMPLATE.md)
- [release/publish-artifacts.toml](../release/publish-artifacts.toml)
- [docs/project-plan.md](./project-plan.md)
