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
