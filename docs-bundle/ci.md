# Continuous integration

Use the same complete lint and test operations as local development. Install a
verified release archive or use the generated Just integration; this
repository does not publish a reusable consumer GitHub Action.

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
