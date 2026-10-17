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
| `SETUP_TOOLS.INSPECTION_UNAVAILABLE` | Dry-run cannot inspect declared tools because the managed helper is unavailable | Run ordinary `just setup` to provision the helper and declared tools, then retry `setup --dry-run`. |
| `CLI.SC_LINT_BACKEND_NOT_FOUND` | A configured lint/test command is missing | Install the named backend, then rerun the profile. |
| `CLI.SC_LINT_INTEGRATION_CONFLICT` | A product-managed integration path contains user-owned changes | Reconcile or move the named file, then rerun `sc-lint init --just`; it will not overwrite consumer-owned content. |
| `CLI.SC_LINT_INTEGRATION_OUTDATED` | One or more required product-managed integration files are absent | Run `sc-lint init --just` to create the named managed files, then rerun the check. |
| `CLI.SC_LINT_DOCS_UNAVAILABLE` | The installed documentation bundle is missing | Install the matching `sc-lint-docs` package and rerun `sc-lint docs`. |
| `CLI.SC_LINT_PROFILE_CONFIG_INVALID` | A consumer lint step, target, selector, or named profile is invalid | Correct the named step or field; a target requires a matching platform and supported literal Cargo command. |

## Initialization conflicts

`sc-lint init --just` reports
`CLI.SC_LINT_INTEGRATION_CONFLICT` rather than overwriting a differing
user-owned `Justfile`, `sc-lint.toml`, `.sc-lint/bootstrap`, or
`.sc-lint/bootstrap.ps1`. Missing managed
files produce `CLI.SC_LINT_INTEGRATION_OUTDATED` in check mode. Move the
conflicting file or reconcile it with [the canonical guide](./just-setup.md),
then rerun initialization.

## Cargo-shear exception migration

The cargo-shear wrapper reads reviewed exceptions from
`.just/lint-config.toml`:

```toml
[cargo_shear.allowed_unused_dependencies]
"sample-app.serde_alias" = "Used by generated serialization code."
```

Use the Cargo package name before the dot and the dependency key after it. If
the manifest renames a dependency with `package = "serde"`, the key is the
manifest alias (here, `serde_alias`); otherwise it is the dependency/package
name. Use `workspace.{dependency_key}` only for a dependency inherited from a
virtual-root workspace dependency table. A package whose Cargo package name is
literally `workspace` is still referred to by its package name, not by the
reserved root.

Previously accepted path-like keys such as `crates/sample-app.serde` are no
longer valid. Replace them with `{package}.{dependency_key}` or, only for a
virtual-root workspace dependency, `workspace.{dependency_key}`. The wrapper
validates that each exception matches an actual dependency and has a reason;
stale keys, missing reasons, path-like keys, and cargo-shear output sections it
cannot parse fail with `SC_LINT_CARGO_SHEAR_CONFIG_INVALID` or a failing check
rather than silently downgrading or passing. An absent exception table
preserves the default findings; a present empty table allows no unused
dependencies. This guide is the offline recovery reference for the compatibility
change.

The cargo-deny adapter retries a completed command failure only when captured
output contains “could not resolve hostname”. It makes at most three attempts
and reports each retry. Policy failures and other command errors are not
retried; the final attempt's output and status determine the result.

## Clean-machine bootstrap

After a repository has committed its generated `sc-lint.toml`, `Justfile`, and
`.sc-lint` helpers, a fresh machine needs only:

```sh
just setup
just lint
```

`just setup` checks for a compatible product installation. If none is
available, it downloads the release selected by the configured floor and
verifies its SHA-256 checksum before activating the complete bundle. A failed
download, checksum, health, or compatibility check does not run lint/test;
follow the reported recovery above. Set
`SC_LINT_INSTALL_DIR` to a writable directory when the default managed
location is unsuitable.

## Getting a diagnostic

```sh
sc-lint compatibility check --config sc-lint.toml --json
sc-lint docs troubleshooting
```

Include the stable code, details, and `sc-lint --json version` output in an
issue; do not paste secrets from CI environments.
