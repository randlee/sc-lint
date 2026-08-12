# Continuous integration

CI should execute the same aggregate contract used locally. A minimal job is:

```yaml
- run: sc-lint init --just
- run: just setup
- run: just lint
- run: just test
```

Most repositories generate the integration during onboarding and commit the
managed files; CI then starts with `just setup`. Pin the release version or
enforce it with `sc-lint.toml`'s `minimum_version`.

## Machine-readable checks

Use `sc-lint --json version` for a stable probe and `--json` on profile commands
when a runner needs to parse the envelope. Preserve stderr on failures so the
stable code and recovery action remain visible in build artifacts.

## Reproducibility

Install a verified release artifact, keep the documentation bundle beside it,
and avoid source-checkout fallbacks. Run lint and test as separate required
steps so a failed profile is actionable. Cache dependencies, not an unverified
sc-lint binary.

See [troubleshooting](./troubleshooting.md) for preflight and backend failures.

## Reusable GitHub Action

For consumer CI, use the versioned Action rather than installing a Cargo
package, cloning this repository for tooling, or copying product scripts. The
Action downloads the named E.5 archive, verifies its SHA-256 entry in the
matching checksum manifest, preflights `sc-lint.toml`, and runs one aggregate
consumer operation.

```yaml
name: sc-lint

on: [pull_request]

permissions:
  contents: read

jobs:
  lint-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: Complete lint profile
        uses: randlee/sc-lint@v1
        with:
          version: 0.5.0
          operation: lint
      - name: Complete test profile
        uses: randlee/sc-lint@v1
        with:
          version: 0.5.0
          operation: test
```

`operation: setup` downloads, verifies, extracts, and compatibility-preflights
the release without running a profile. It exposes `binary-path`, `docs-path`,
and `version` outputs, and adds the binary directory to `PATH` for a later
step.

## Pinning, cache, and offline runners

`randlee/sc-lint@v1` is the supported stable-major form. For a reproducible or
high-assurance workflow, pin the Action to a reviewed immutable full commit
SHA and keep `version` explicit:

```yaml
- uses: randlee/sc-lint@0123456789abcdef0123456789abcdef01234567
  with:
    version: 0.5.0
    operation: lint
```

The Action needs no token beyond the job's normal checkout permissions; keep
`contents: read` unless another consumer step needs more. The `version` input
identifies the product release, while `@v1` or the Action SHA identifies the
Action code.

For a verified internal mirror or hermetic fixture, pass both `artifact-url`
and `checksums-url`. The archive must retain its normal E.5 filename and its
matching checksum entry. Cache dependencies or a verified mirror, not a binary
copied from another job. An unavailable release produces
`ACTION.SC_LINT_ARTIFACT_UNAVAILABLE`; it never changes to a package manager or
source build.

The shipped `sc-lint-docs` bundle is available without network access through
the `docs-path` output, and the Action checks `sc-lint docs --path` after
extraction. Preserve stderr in CI logs: it carries the stable error code and
recovery text for artifact, checksum, compatibility, or profile failure.
