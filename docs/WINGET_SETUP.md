# Winget Setup

Winget publishing is an active post-release channel for `sc-lint`. This page
describes how the channel is wired and what must be in place before it runs.

## Channel Facts

Everything below is read from `release/publish-artifacts.toml`
(`[channels.winget]`) and `.github/workflows/winget-publish.yml`. Change those
files, not this page, when the values move.

- Package identifier: `randlee.sc-lint`
- Installer targets:
  - `x86_64-pc-windows-msvc` (x64)
  - `aarch64-pc-windows-msvc` (ARM64)
- Installer assets: `sc-lint_<version>_<target>.zip` on the GitHub release
  for the tag. The asset regexes come from the channel config, not from a
  hand-maintained list.
- Workflow: `winget-publish.yml` (`workflow_dispatch`, input `tag`, for example
  `v0.8.5`).

## Release Flow

1. The release is published first. The workflow's `verify-release` job checks
   that the release assets matching the winget patterns exist before anything
   is submitted.
2. The `publish-winget` job's `winget_probe` step fails closed:
   - A confirmed 404 for `manifests/r/randlee/sc-lint/<version>` in
     `microsoft/winget-pkgs` means the version is new and submission continues.
   - An existing manifest, or an open or merged `winget-pkgs` pull request for
     the same identifier and version, skips submission.
   - Any other probe result (auth failure, rate limit, transient API error,
     incomplete search) fails the leg instead of risking a duplicate pull
     request.
3. The job submits the installer manifests with
   `vedantmgoyal2009/winget-releaser` (pinned to a commit SHA) to
   `microsoft/winget-pkgs`.

A retried run for the same tag is safe: the probe in step 2 turns it into a
skip when the submission already exists.

## Required Setup

- Repository secret `WINGET_GITHUB_TOKEN`: a token that can push a branch to a
  fork of `microsoft/winget-pkgs` and open a pull request against it. The
  workflow does not use the repository `GITHUB_TOKEN` for this step, by design.
- A published GitHub release for the tag with both Windows zip archives.

## Verifying a Submission

- Check the generated installer manifest for `Architecture: x64` and
  `Architecture: arm64`, and confirm the ARM64 installer URL points at the
  `aarch64-pc-windows-msvc` archive. The ARM64 mapping has not been exercised by
  a live submission yet, so check it on the first ARM64 submission.

## Related Docs

- [docs/release-tag-protection.md](./release-tag-protection.md)
- [release/publish-artifacts.toml](../release/publish-artifacts.toml)
