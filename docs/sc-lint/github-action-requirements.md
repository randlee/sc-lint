# GitHub Action Requirements

This document specifies the reusable `sc-lint` GitHub Action required by
`REQ-PRODUCT-022`. The Action lives at the repository root and is adopted as
`randlee/sc-lint@v1`. It is a release-artifact consumer: it is not a Cargo,
source-checkout, or analyzer-package interface.

## Contract

| ID | Requirement | Validation reference |
| --- | --- | --- |
| GA-001 | The Action must obtain only a named `sc-lint` release archive and its sibling `checksums.txt` from the release download location. It selects the archive for Linux x86_64, macOS x86_64/aarch64, or Windows x86_64. | [`action.yml`](../../action.yml), [`action/test/action.test.cjs`](../../action/test/action.test.cjs) |
| GA-002 | Before extracting an archive, the Action must verify its SHA-256 digest against the entry for that exact archive in `checksums.txt`. | [`action/index.js`](../../action/index.js), `checksum mismatch` fixture in [`action/test/action.test.cjs`](../../action/test/action.test.cjs) |
| GA-003 | The Action accepts `operation` values `setup`, `lint`, and `test`. `setup` installs and preflights the release; `lint` and `test` run the E.3 consumer commands only after the E.1 compatibility preflight. | [`action.yml`](../../action.yml), operation fixtures in [`action/test/action.test.cjs`](../../action/test/action.test.cjs) |
| GA-004 | The Action must run `sc-lint --config <config> compatibility check --binary <installed binary>` before a consumer command. It must fail when the configured minimum is incompatible. | [`action/index.js`](../../action/index.js), incompatible-floor fixture in [`action/test/action.test.cjs`](../../action/test/action.test.cjs) |
| GA-005 | The Action publishes `binary-path`, `docs-path`, and `version` outputs. The extracted binary directory is added to the runner path; `docs-path` must be the shipped offline `sc-lint-docs` directory. | [`action.yml`](../../action.yml), output and offline-docs fixture in [`action/test/action.test.cjs`](../../action/test/action.test.cjs) |
| GA-006 | Failures have stable codes and recovery: `ACTION.SC_LINT_ARTIFACT_UNAVAILABLE`, `ACTION.SC_LINT_CHECKSUM_MISMATCH`, `ACTION.SC_LINT_COMPATIBILITY_FAILED`, and `ACTION.SC_LINT_COMMAND_FAILED`. | [`action/index.js`](../../action/index.js), failure fixtures in [`action/test/action.test.cjs`](../../action/test/action.test.cjs), [troubleshooting](../../docs-bundle/troubleshooting.md) |
| GA-007 | The stable adoption form is `randlee/sc-lint@v1`. Security-sensitive consumers must pin the exact immutable action commit; the product release is always selected from `config-path`'s `minimum_version`, while an optional `version` input can only assert equality. Release publication refreshes the movable `v1` Action tag only after the release is published. | [CI guide](../../docs-bundle/ci.md), [release workflow](../../.github/workflows/release.yml) |
| GA-008 | The Action supports an explicit local `artifact-url` and `checksums-url` for hermetic fixtures/offline mirrors; it never silently falls back to a package manager, Cargo, a source checkout, or an analyzer package name. | [`action.yml`](../../action.yml), all-platform fixture in [`action/test/action.test.cjs`](../../action/test/action.test.cjs), [CI guide](../../docs-bundle/ci.md) |

## Planned Phase F version-authority amendment

Phase F (`REQ-PRODUCT-025`) replaces the independent required `version` input
with config-derived selection: the Action parses
`[tool.sc-lint].minimum_version` from `config-path` and uses that exact value
for its archive and compatibility preflight. A transitional `version` input,
if retained, is optional assertion-only and must fail on semantic mismatch; it
cannot select an archive. F.4b owns implementation and updates the input
table, fixtures, CI guide, and release behavior together.

## Inputs and outputs

| Input | Required | Default | Meaning |
| --- | --- | --- | --- |
| `operation` | no | `lint` | One of `setup`, `lint`, or `test`. |
| `version` | no | — | Transitional assertion-only SemVer, without a leading `v`; it must equal the `minimum_version` read from `config-path` and cannot select an archive. |
| `config-path` | no | `sc-lint.toml` | Consumer configuration passed to compatibility and operation commands. |
| `artifact-url` | no | Derived from `config-path`'s minimum version, platform, and release base URL | Exact archive URL; intended for release mirrors and local fixtures. |
| `checksums-url` | no | Derived from `config-path`'s minimum version and release base URL | URL of the matching checksum manifest. |
| `release-base-url` | no | `https://github.com/randlee/sc-lint/releases/download` | Trusted release download root. |
| `working-directory` | no | `.` | Directory containing the consumer configuration and profiles. |

The Action provides `binary-path`, `docs-path`, and `version` outputs. It also
adds the binary parent directory to `GITHUB_PATH`; consumers should prefer the
explicit outputs when passing paths to a later tool.

## Provenance, cache, and offline policy

The checksum manifest is fetched from the same release version as the archive;
the Action verifies the exact archive name before extraction. A cache may retain
the already verified Action install directory, but no cache entry is trusted as
a binary source without the digest check in its producing Action invocation.

After extraction, documentation discovery is entirely offline: the Action
exposes the archive's `sc-lint-docs` directory and validates it through
`sc-lint docs --path`. Offline runners must supply a reachable internal mirror
through both explicit URL inputs or pre-stage a verified archive/mirror; the
Action reports `ACTION.SC_LINT_ARTIFACT_UNAVAILABLE` rather than using another
installation mechanism.

## Pinning policy

Use `randlee/sc-lint@v1` for normal managed adoption. For a reproducible or
high-assurance workflow, pin the `uses:` reference to an immutable full commit
SHA. The Action always selects the verified release artifact from
`[tool.sc-lint].minimum_version` in `config-path`; an optional `version` value
can only assert that the workflow agrees with that configuration. The Action
major tag identifies the action interface, while `sc-lint.toml` identifies the
verified release artifact; neither is inferred from Cargo metadata or a
checkout.
