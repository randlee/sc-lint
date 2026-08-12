# Best practices

## One contract for people and agents

Keep `sc-lint.toml`, the generated Just integration, and the product version
floor in version control. Agents should begin with `just setup`, make changes,
and finish with `just lint` and `just test`. They do not need to learn Cargo
package topology.

## Make failures early and actionable

Run the compatibility preflight before expensive work. Keep lint and test
profiles complete and explicit; avoid fast aliases that omit required checks.
Use stable JSON envelopes at automation boundaries and retain the human guide
reference in logs.

## Keep ownership clear

The product owns installation, upgrade, diagnostics, and documentation. The
consumer owns its README, source policy, and profile command lists. A generated
file may be regenerated, but a user-owned file must never be silently replaced.

## CI and release hygiene

Install only checksum-verified artifacts, pin a minimum compatible version, and
keep the docs bundle version-matched with the binary. Validate package-guide
completeness and relative links before publishing. Use `sc-lint docs --path`
to make the installed layout observable to release automation.
