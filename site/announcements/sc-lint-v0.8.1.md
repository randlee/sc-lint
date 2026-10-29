# sc-lint v0.8.1 — More Precise Checks and Better Consumer Test Profiles

**Release:** 0.8.1 · **Install:** `brew install randlee/tap/sc-lint` (Homebrew), `pip install sc-lint` (PyPI), `cargo install sc-lint` (crates.io), or the platform archives on the [release page](https://github.com/randlee/sc-lint/releases/tag/v0.8.1)

[Changelog](https://github.com/randlee/sc-lint/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-lint/releases/tag/v0.8.1) · [Operator manual](https://github.com/randlee/sc-lint/blob/main/docs-bundle/README.md)

---

## More Precise Rust Workspace Checks

Phase L improves portability and boundary analysis across real workspaces. Portability checks account for additional platform `cfg` forms, recognize the `USER` to `USERNAME` fallback, and avoid false positives for Windows namespace paths and private helpers in paired platform-adapter files. Boundary checks use each owner's `allowed_dependents` policy for external composition roots and apply path exclusions to path-less cycle findings using the source files behind those findings.

The runtime analyzer adds `SCB-RUNTIME-003` for blocking `std::thread::sleep` in production targets of packages that declare Tokio. Replace those calls with `tokio::time::sleep(...).await`; test-only code is outside the rule's scope.

Rust analyzer reports are checked against the bundled `analyzer-report-v1` JSON Schema. Python adapter responses are also checked against the stable `sc-lint-python-v1` contract, so malformed backend data fails with an actionable protocol error.

---

## Consumer Test Profiles

Consumer test configuration can use named command and group selectors, choose a default, and set a selector's environment, working directory, and platform scope. Selectors forward arguments to their command, and `sc-lint test --list` shows the available names.

Lint and test steps run concurrently and report all failures in declaration order. Cargo commands remain serialized, and `--fail-fast` stops starting eligible steps after the first failure.

---

## Adoption and Tooling Updates

Installer-rendered adoption templates now use `.tmpl`; `.j2` remains for templates rendered by sc-compose. Version-sync checks can validate Python manifests and configurable Winget and release-wiring fields. Cargo-shear recognizes Rust test fixtures consumed by trybuild `.pass()` and `.compile_fail()` calls. GitHub Action inputs accept hyphenated names such as `working-directory` and `config-path`.

---

## Upgrade Notes

The new runtime rule can flag existing blocking sleeps in production Tokio packages. Replace those calls with asynchronous Tokio sleeps or exclude reviewed migration paths using the runtime analyzer's existing path-exclusion configuration. Consumer lint and test steps may now run in parallel; use `--fail-fast` when a consumer wants to stop scheduling eligible work after the first failure.
