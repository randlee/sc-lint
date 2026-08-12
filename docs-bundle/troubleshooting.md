# Troubleshooting

Every installation, compatibility, and consumer backend failure is a stable
machine-readable error. The human form includes the same recovery guidance.

## Compatibility and installation codes

| Code | Cause | Recovery |
| --- | --- | --- |
| `CLI.SC_LINT_CONFIG_MISSING` | `sc-lint.toml` or its tool section is absent | Run `sc-lint init --just`, then edit the minimum version. |
| `CLI.SC_LINT_CONFIG_MALFORMED` | The minimum version or profile shape is invalid | Fix the named field and rerun `sc-lint compatibility check`. |
| `CLI.SC_LINT_BINARY_NOT_FOUND` | The configured product binary is unavailable | Run `just setup` or install a verified release. |
| `CLI.SC_LINT_BINARY_EXECUTION_FAILED` | The binary could not execute | Check permissions/platform and reinstall with `just setup`. |
| `CLI.SC_LINT_VERSION_PROBE_MALFORMED` | The version probe was not the stable schema | Replace the binary with a matching release. |
| `CLI.SC_LINT_VERSION_UNPARSABLE` | The observed version is not SemVer | Reinstall a supported release. |
| `CLI.SC_LINT_VERSION_TOO_OLD` | Installed version is below the configured floor | Run `just upgrade` or raise/lower the declared floor intentionally. |
| `CLI.SC_LINT_INSTALL_UNSUPPORTED_PLATFORM` | Host is outside the release matrix | Use a supported host or build/install through a documented platform path. |
| `CLI.SC_LINT_RELEASE_UNAVAILABLE` | Release archive or checksum could not be obtained | Retry with network access or stage the release manually. |
| `CLI.SC_LINT_RELEASE_CHECKSUM_MISMATCH` | Downloaded bytes do not match the manifest | Delete the staging file and download again from the release source. |
| `CLI.SC_LINT_INSTALL_PERMISSION_DENIED` | Managed install directory is not writable | Choose a writable managed directory and rerun setup. |
| `CLI.SC_LINT_POST_INSTALL_VERSION_FAILED` | Activated binary failed its version probe | The previous binary should remain active; retry with a verified archive. |
| `CLI.SC_LINT_INSTALL_ROLLBACK_FAILED` | Previous binary could not be verified after failure | Use the reported backup path and restore it manually before retrying. |
| `CLI.SC_LINT_INSTALL_ACTIVATION_FAILED` | Atomic activation failed | Choose a writable install directory and retry. |
| `CLI.SC_LINT_BACKEND_NOT_FOUND` | A configured lint/test command is missing | Install the named backend, then rerun the profile. |
| `CLI.SC_LINT_INTEGRATION_CONFLICT` | A product-managed integration path contains user-owned changes | Reconcile or move the named file, then rerun `sc-lint init --just`; it will not overwrite consumer-owned content. |
| `CLI.SC_LINT_INTEGRATION_OUTDATED` | One or more required product-managed integration files are absent | Run `sc-lint init --just` to create the named managed files, then rerun the check. |
| `CLI.SC_LINT_DOCS_UNAVAILABLE` | The installed documentation bundle is missing | Install the matching `sc-lint-docs` package and rerun `sc-lint docs`. |
| `ACTION.SC_LINT_ARTIFACT_UNAVAILABLE` | The Action could not obtain the selected release archive or checksum manifest | Check the exact `version` and release/mirror URLs; make the verified release reachable, then retry. |
| `ACTION.SC_LINT_CHECKSUM_MISMATCH` | The Action archive digest differs from the selected `checksums.txt` entry | Discard the archive and retry from the trusted release or mirror; do not bypass verification. |
| `ACTION.SC_LINT_COMPATIBILITY_FAILED` | The extracted release fails the configured E.1 minimum-version preflight | Select a release meeting `minimum_version` or intentionally reconcile `sc-lint.toml`. |
| `ACTION.SC_LINT_COMMAND_FAILED` | The selected consumer setup, lint, test, or offline-documentation command failed | Inspect the preserved command output, repair the named consumer profile or bundle, then retry. |

## Initialization conflicts

`sc-lint init --just` reports
`CLI.SC_LINT_INTEGRATION_CONFLICT` rather than overwriting a differing
user-owned `Justfile`, `sc-lint.toml`, or `.sc-lint/bootstrap`. Missing managed
files produce `CLI.SC_LINT_INTEGRATION_OUTDATED` in check mode. Move the
conflicting file or reconcile it with [the canonical guide](./just-setup.md),
then rerun initialization.

## Getting a diagnostic

```sh
sc-lint compatibility check --config sc-lint.toml --json
sc-lint docs troubleshooting
```

Include the stable code, details, and `sc-lint --json version` output in an
issue; do not paste secrets from CI environments.
