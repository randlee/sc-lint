set windows-shell := ["pwsh", "-NoLogo", "-Command"]

python_cmd := if os_family() == "windows" { "python" } else { "python3" }
sc_lint_binary := if os_family() == "windows" { ".\\target\\debug\\sc-lint.exe" } else { "./target/debug/sc-lint" }
export SC_LINT_BIN := sc_lint_binary

# Show the curated repo task help.
default: help

# Show the curated repo task help.
help:
    {{python_cmd}} .just/print_help.py

[private]
_fmt-write:
    cargo fmt --all

[private]
_fmt-check:
    cargo fmt --all --check

# Format the Rust workspace or run the formatting gate.
fmt mode='check':
    {{python_cmd}} .just/run_fmt.py {{mode}}

[private]
_lint-fmt:
    @just fmt check

[private]
_lint-clippy:
    cargo clippy --workspace --all-targets -- -D warnings

[private]
_lint-modules:
    {{python_cmd}} .just/lint_cargo_modules.py

[private]
_lint-deny:
    {{python_cmd}} .just/lint_cargo_deny.py

[private]
_lint-shear:
    {{python_cmd}} .just/lint_cargo_shear.py

[private]
_lint-sc-boundary:
    {{python_cmd}} .just/lint_sc_boundary.py

[private]
_lint-sc-portability:
    {{python_cmd}} .just/lint_sc_portability.py

[private]
_lint-manifests:
    {{python_cmd}} .just/lint_manifests.py

# Verify crate/release versions stay aligned.
[private]
_lint-version:
    {{python_cmd}} .just/check_version_sync.py

# Show current workspace version state or latest recommended direct dependency upgrades.
version mode='current':
    {{python_cmd}} .just/run_version.py {{mode}}

[private]
_lint-spell:
    {{python_cmd}} .just/lint_codespell.py

[private]
_lint-pytests:
    {{python_cmd}} .just/run_pytests.py

# Build the full workspace.
build:
    cargo build --workspace

# Build the product binary used by this repository's consumer-model recipes.
[private]
_source-build:
    cargo build --bin sc-lint

# Run the complete source-maintainer test suite behind the consumer profile.
[private]
_source-test:
    cargo build --workspace
    cargo test --workspace
    {{python_cmd}} .just/run_pytests.py
    node --test action/test/action.test.cjs

# Remove workspace build artifacts.
clean:
    cargo clean

# Run the complete source-maintainer lint suite behind the consumer profile.
[private]
_source-lint target='full':
    {{python_cmd}} .just/run_lint.py {{target}}

# Verify the root model's compatible product binary and report setup state.
setup: _source-build
    .sc-lint/bootstrap setup --config sc-lint.toml --dry-run

# Run every required source lint gate through the consumer contract.
lint: _source-build
    .sc-lint/bootstrap lint --config sc-lint.toml

# Run every required source test gate through the consumer contract.
test: _source-build
    .sc-lint/bootstrap test --config sc-lint.toml

# Inspect the managed upgrade path without changing the source checkout.
upgrade: _source-build
    .sc-lint/bootstrap upgrade --config sc-lint.toml --check --dry-run
