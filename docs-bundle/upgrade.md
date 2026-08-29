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

## Upgrading an adopted integration

For an established repository, upgrade first does not rewrite its managed
configuration. Regenerate and review a configure plan whenever the repository
or requested setup choices changed:

```sh
sc-lint --json --root . configure --request sc-lint-request.json > sc-lint-plan.json
sc-lint --json --root . configure --request sc-lint-request.json --apply --plan sc-lint-plan.json
just upgrade
```

The apply step rejects stale source digests and does not overwrite a
consumer-owned `Justfile` recipe. It may remove only the documented exact
legacy sc-compose 0.4 bundle through the reviewed transaction: both old
composite actions, the copied `.just` helper bundle/runtime marker, and the
old materialization script. Every one must match its recorded digest and the
new config/bootstrap/Just replacements must be valid; similarly named or
partial files are left untouched.
