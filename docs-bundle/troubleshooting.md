# Troubleshooting

Every installation, compatibility, and consumer backend failure is a stable
machine-readable error. The human form includes the same recovery guidance.

## Compatibility and installation codes

| Code | Cause | Recovery |
| --- | --- | --- |
| `CLI.SC_LINT_CONFIG_MISSING` | `sc-lint.toml` or its tool section is absent | Run `sc-lint init --just`, then edit the minimum version. |
| `CLI.SC_LINT_CONFIG_MALFORMED` | The minimum version or profile shape is invalid | Fix the named field and rerun `sc-lint compatibility check`. |
| `CLI.SC_LINT_BINARY_NOT_FOUND` | A direct product command could not resolve the configured binary | Use `just setup` to install the configured verified release, or set `SC_LINT_BIN` to a compatible executable. |
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
user-owned `Justfile`, `sc-lint.toml`, `.sc-lint/bootstrap`, or
`.sc-lint/bootstrap.ps1`. Missing managed
files produce `CLI.SC_LINT_INTEGRATION_OUTDATED` in check mode. Move the
conflicting file or reconcile it with [the canonical guide](./just-setup.md),
then rerun initialization.

## Configure apply conflicts

`sc-lint configure --apply --plan <file>` uses the configure recovery family:

| Code | Meaning | Recovery |
| --- | --- | --- |
| `CLI.CONFIGURE_STALE_PLAN` | The request, plan identifier, or a planned source file changed after review. | Regenerate the plan, review it again, then apply the new file. |
| `CLI.CONFIGURE_UNMANAGED_COLLISION` | A marker block is malformed, a root `Justfile` recipe would be shadowed, or a legacy removal is not an exact allowlisted fingerprint. | Review the exportable patch and reconcile the user-owned integration; no file was written. |
| `CLI.CONFIGURE_ROLLBACK_FAILED` | A failed transaction could not restore every target. | Restore the listed backup paths, repair permissions, and regenerate the plan. |

An established `Justfile` is safe only when its managed block is exactly the
documented begin marker, import, and end marker. `configure` preserves CRLF
and all bytes outside that range. Do not hand-edit the block: remove it and
regenerate/review a plan instead.

## Clean-machine bootstrap

After a repository has committed its generated `sc-lint.toml`, `Justfile`, and
`.sc-lint` helpers, a fresh machine needs only:

```sh
just setup
just lint
```

The helpers resolve `SC_LINT_BIN`, the managed install, and `PATH` in that
order. If none exists, `just setup` downloads the exact configured release and
verifies its SHA-256 checksum before activation. A failed download or checksum
does not run lint/test; follow the reported release code above. Set
`SC_LINT_INSTALL_DIR` to a writable directory when the default managed
location is unsuitable.

## Getting a diagnostic

```sh
sc-lint compatibility check --config sc-lint.toml --json
sc-lint docs troubleshooting
```

Include the stable code, details, and `sc-lint --json version` output in an
issue; do not paste secrets from CI environments.
