# Continuous integration

Use the same complete lint and test operations as local development. Use the
shipped reusable GitHub Action, `randlee/sc-lint@v1`, for managed adoption; or
install a verified release archive; or use the generated Just integration.

## Reusable GitHub Action

`randlee/sc-lint@v1` installs a verified release archive, checks the checksum
against the release's `checksums.txt`, runs the compatibility preflight, and
then runs the consumer contract. Choose the operation with `operation`: `setup`
installs and preflights the release, while `lint` (the default) and `test` run
the consumer commands after the preflight.

```yaml
- uses: randlee/sc-lint@v1
  with:
    operation: lint
    version: 0.7.3
```

`version` is required. It is the exact released product SemVer, without a
leading `v`, and it selects the verified artifact. Do not infer it from Cargo
metadata or a checkout.

For a reproducible or high-assurance workflow, pin `uses:` to an immutable full
commit SHA and keep `version` explicit:

```yaml
- uses: randlee/sc-lint@<full-commit-sha>
  with:
    operation: test
    version: 0.7.3
```

The `v1` tag identifies the Action interface and is a movable tag, refreshed
after each release is published. The `version` input identifies the release
artifact. Neither implies the other.

### Action inputs

| Input | Required | Default | Description |
| --- | --- | --- | --- |
| `operation` | No | `lint` | Selects `setup`, `lint`, or `test`. `setup` installs and preflights only. |
| `version` | Yes | — | Exact released sc-lint SemVer without a leading `v`; selects the verified release artifact. |
| `config-path` | No | `sc-lint.toml` | Consumer configuration path, relative to `working-directory`. |
| `artifact-url` | No | Derived from `release-base-url` and `version` | Exact release archive URL, for a mirror or offline runner. |
| `checksums-url` | No | Derived from `release-base-url` and `version` | URL of the matching `checksums.txt` manifest. |
| `release-base-url` | No | `https://github.com/randlee/sc-lint/releases/download` | Trusted release download root used to derive the archive and checksum URLs. |
| `working-directory` | No | `.` | Consumer working directory containing `sc-lint.toml`; relative `config-path` values resolve from here. |

When overriding a mirror or offline location, provide `artifact-url` and
`checksums-url` together. They must identify the exact archive selected by
`version` and the matching checksum manifest. When neither is supplied,
`release-base-url` is used to derive both URLs.

### Action outputs

| Output | Description |
| --- | --- |
| `binary-path` | Absolute path of the verified `sc-lint` binary. |
| `docs-path` | Absolute path of the shipped offline `sc-lint-docs` directory. Documentation discovery never reaches the network. |
| `version` | Verified release version selected by the Action. |

The Action does not fall back to a package manager, Cargo, a source checkout,
or an analyzer package name. When an artifact cannot be reached, it reports
`ACTION.SC_LINT_ARTIFACT_UNAVAILABLE`.

Stable failure codes are `ACTION.SC_LINT_ARTIFACT_UNAVAILABLE`,
`ACTION.SC_LINT_CHECKSUM_MISMATCH`, `ACTION.SC_LINT_COMPATIBILITY_FAILED`, and
`ACTION.SC_LINT_COMMAND_FAILED`. Each has a recovery step in
[troubleshooting](./troubleshooting.md).

The repository's `.github/actions/*` composites are repository-only workflow
components for this project's own CI. Consumers use `randlee/sc-lint@v1`, not
those paths.

## Machine-readable checks

Use `sc-lint --json version` for a stable probe and `--json` on profile commands
when a runner needs to parse the envelope. Preserve stderr on failures so the
stable code and recovery action remain visible in build artifacts.

## Reproducibility

Preserve the archive's declared documentation-bundle path alongside its
executables when installing by hand.
Run lint and test as separate required steps so a failed profile is actionable.
Cache dependencies, not an unverified sc-lint binary. See
[troubleshooting](./troubleshooting.md) for preflight and backend failures.

## Consumer workflow

Generate and commit the managed files during onboarding. A clean CI runner can
then use the same documented local sequence without a preinstalled binary:

```yaml
- run: just setup
- run: just lint
- run: just test
```

`just setup` uses the declared minimum version, verifies the matching release
checksum, and activates the complete managed product before later recipes run.
Stable errors and recovery guidance are written to stderr for setup,
preflight, and backend failures.
The source-maintainer `pytests` task runs only tests bundled in the wheel; it
does not collect tests from the consumer checkout. See the
[detailed test-runner note](./best-practices.md#bundled-python-tests).

The Rust consumer executor supports named lint profiles and platform scopes. A
profile is an ordered list of existing lint step names, not a nested profile.
`--platform-scope host` is the default; `all` reports unsupported cross-platform
selections as skips, while a named platform selects its tagged steps. Only
literal cargo `check` and `clippy` commands can declare a target. Native runs
use Cargo; Linux/macOS to Windows MSVC x86_64 uses `cargo xwin`. Other cross
target combinations are reported as unsupported, and a missing tool or failed
supported command fails the run. See [using sc-lint](./using-sc-lint.md) for
examples and output behavior.
