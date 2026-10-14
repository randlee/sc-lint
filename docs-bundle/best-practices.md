# Best practices

## One contract for people and agents

Keep `sc-lint.toml`, the generated Just integration, and the product version
floor in version control. Agents should begin with `just setup` when needed,
make changes, and finish with exactly `just lint` and `just test`. They do not
need to learn Cargo package topology.

## Make failures early and actionable

Run the compatibility preflight before expensive work. Keep lint and test
profiles complete and explicit; avoid fast aliases that omit required checks.
Use stable JSON envelopes at automation boundaries and retain the human guide
reference in logs.

The built-in `ci` and `full` source-maintainer lint profiles include the
runtime analyzer. The packaged `sc-lint` Python wheel runs only its bundled
test suite; it does not assume the checkout is the sc-lint repository. Reviewed
cargo-shear exceptions belong in its configuration with a reason and a
canonical package/dependency key. An exception does not change the analyzer's
underlying unused-dependency result.

## Keep ownership clear

The product owns installation, upgrade, diagnostics, and documentation. The
consumer owns its README, source policy, and profile command lists. A generated
file may be regenerated, but a user-owned file must never be silently replaced.

The 0.7.1 identity-literals check recognizes an identity assembled from literal
pieces in `concat!`, including repeated occurrences. The Windows Python runner
keeps captured output readable on restrictive consoles by replacing only
characters the active encoding cannot write. Template rendering rejects
unresolved placeholders, including placeholders written without spaces. The
0.7.3 launcher rejects a stale or foreign `sc-lint-*` executable found on
`PATH` instead of running it; use `sc-lint setup` to install a matching bundle.
Human verdict text, JSON status, and process exit status agree.

## CI and release hygiene

Install only checksum-verified artifacts, pin a minimum compatible version, and
keep the docs bundle version-matched with the binary. Validate package-guide
completeness and relative links before publishing. Use `sc-lint docs --path`
to make the installed layout observable to release automation.
