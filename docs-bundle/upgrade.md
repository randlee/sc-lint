# Upgrade and rollback

The configured minimum version is a floor, not a request to downgrade. A
current or newer compatible installation is a no-op.

```sh
sc-lint upgrade --check --config sc-lint.toml
sc-lint upgrade --dry-run --config sc-lint.toml
just upgrade
```

The installer selects a host release, downloads the archive and checksum
manifest, verifies the digest, stages extraction, and atomically activates the
binary. A failed post-install version probe retains the previous working
binary. If restoration cannot be verified, the command reports a distinct
rollback failure and identifies the manual backup location.

After upgrading, verify both product and docs:

```sh
sc-lint --json version
sc-lint docs --path
```

Keep `sc-lint.toml` and the consumer README under consumer ownership; upgrade
does not rewrite either file.
