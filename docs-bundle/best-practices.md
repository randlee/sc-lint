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
underlying unused-dependency result. See the
[cargo-shear migration guide](./troubleshooting.md#cargo-shear-exception-migration)
for accepted key forms, parser failures, and recovery.

## Keep ownership clear

The product owns installation, upgrade, diagnostics, and documentation. The
consumer owns its README, source policy, and profile command lists. A generated
file may be regenerated, but a user-owned file must never be silently replaced.

## Shipped behavior notes

### Bundled Python tests

The source-maintainer `pytests` task runs the Python unit tests packaged inside
the installed `sc_lint` wheel. It locates the wheel's sibling `tests/` folder
from the installed module, so it does not assume a source checkout or collect
the consumer repository's tests. The task prints fixture counts and returns a
failing status when a bundled test fails. It complements the consumer's
separately declared `just test` profile; it does not replace it. See the
[CI guide](./ci.md#consumer-workflow) for the consumer commands.

### Identity literals and captured output

The 0.7.1 `identity-literals` check joins Rust string-literal arguments in a
literal-only `concat!(...)` and checks the resulting value. It reports distinct
repeated occurrences and ignores text inside comments and ordinary string
literals. It does not evaluate arbitrary macro expressions or a `concat!` that
contains non-literal arguments. The source maintainer's identity-literal
command and configuration remain separate from consumer lint profiles.

The Windows Python runner writes captured UTF-8 child output using the active
console encoding. If the console cannot represent some characters, it replaces
only those characters and leaves the rest of the output unchanged. This keeps
the original diagnostics readable on restrictive Windows consoles.

### Template rendering

The adoption installer checks rendered templates for unresolved variables
before writing files. A leftover placeholder such as `{{name}}` is an error
even when the placeholder has no spaces; correct the template or its input
instead of accepting a partly rendered consumer file. The
[Just setup guide](./just-setup.md#generate-the-integration) describes the
generated files and conflict behavior.

### Binary resolution and PATH fallback

The consumer bootstrap uses an explicit `SC_LINT_BIN` when supplied, then the
managed installation, then a compatible `sc-lint` on `PATH`. If the `PATH`
candidate is incompatible with the configured minimum version, setup treats
it as unavailable and installs the pinned release; a missing managed path is
also treated as unavailable. A stale or foreign `sc-lint-*` analyzer companion
found on `PATH` is rejected with `CLI.BACKEND_EXEC_FAILURE`; run `sc-lint setup`
to install the matching bundle instead of running that companion. See
[installation](./installation.md) for the supported setup path.

### Canonical package identity

Boundary inventory references resolve to a Cargo package name first, then to
a unique workspace-member directory, then to a unique library target name.
After resolution, package policy uses the canonical Cargo package identity; an
ambiguous or unknown reference fails instead of being skipped. The
[boundary package guide](./packages/sc-lint-boundary.md#canonical-package-identity)
is the canonical detailed explanation and gives the exact fallback rules.

Human verdict text, JSON status, and process exit status agree. See
[using sc-lint](./using-sc-lint.md#inspecting-output) for the consumer-facing
status codes.

## CI and release hygiene

Install only checksum-verified artifacts, pin a minimum compatible version, and
keep the docs bundle version-matched with the binary. Validate package-guide
completeness and relative links before publishing. Use `sc-lint docs --path`
to make the installed layout observable to release automation.
