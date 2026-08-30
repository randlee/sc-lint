set windows-shell := ["pwsh", "-NoLogo", "-Command"]

host_python := if os_family() == "windows" { "python" } else { "python3" }
# Source recipes run the sc_lint helper package from the repo-local venv (G.3a).
python_cmd := if os_family() == "windows" { ".\\.sc-lint\\venv\\Scripts\\python.exe" } else { ".sc-lint/venv/bin/python3" }
sc_lint_binary := if os_family() == "windows" { ".\\target\\debug\\sc-lint.exe" } else { "./target/debug/sc-lint" }
export SC_LINT_BIN := sc_lint_binary

# Show the curated repo task help.
default: help

# Show the curated repo task help.
help: _source-venv
    {{python_cmd}} -m sc_lint.print_help

[private]
_fmt-write:
    cargo fmt --all

[private]
_fmt-check:
    cargo fmt --all --check

# Format the Rust workspace or run the formatting gate.
fmt mode='check': _source-venv
    {{python_cmd}} -m sc_lint.run_fmt {{mode}}

[private]
_lint-fmt:
    @just fmt check

[private]
_lint-clippy:
    cargo clippy --workspace --all-targets -- -D warnings

[private]
_lint-modules:
    {{python_cmd}} -m sc_lint.lint_cargo_modules

[private]
_lint-deny:
    {{python_cmd}} -m sc_lint.lint_cargo_deny

[private]
_lint-shear:
    {{python_cmd}} -m sc_lint.lint_cargo_shear

[private]
_lint-sc-boundary:
    {{python_cmd}} -m sc_lint.lint_sc_boundary

[private]
_lint-sc-portability:
    {{python_cmd}} -m sc_lint.lint_sc_portability

[private]
_lint-manifests:
    {{python_cmd}} -m sc_lint.lint_manifests

# Verify crate/release versions stay aligned.
[private]
_lint-version:
    {{python_cmd}} -m sc_lint.check_version_sync

# Show current workspace version state or latest recommended direct dependency upgrades.
version mode='current': _source-venv
    {{python_cmd}} -m sc_lint.run_version {{mode}}

[private]
_lint-spell:
    {{python_cmd}} -m sc_lint.lint_codespell

[private]
_lint-pytests:
    {{python_cmd}} -m sc_lint.run_pytests

# Build the full workspace.
build:
    cargo build --workspace

# Build the product binary used by this repository's consumer-model recipes.
[private]
_source-build:
    cargo build --bin sc-lint

# Provision .sc-lint/venv with the sc_lint helper wheel built from this checkout.
[private]
_source-venv:
    {{host_python}} bindings/sc-lint-py/python/sc_lint/source_venv.py

# Run the complete source-maintainer test suite behind the consumer profile.
[private]
_source-test:
    cargo build --workspace
    cargo test --workspace
    {{python_cmd}} -m sc_lint.run_pytests
    node --test action/test/action.test.cjs

# Remove workspace build artifacts.
clean:
    cargo clean

# Run the complete source-maintainer lint suite behind the consumer profile.
[private]
_source-lint target='full':
    {{python_cmd}} -m sc_lint.run_lint {{target}}

# Verify the root model's compatible product binary and report setup state.
setup: _source-build _source-venv
    .sc-lint/bootstrap setup --config sc-lint.toml --dry-run

# Run every required source lint gate through the consumer contract.
lint: _source-build _source-venv
    .sc-lint/bootstrap lint --config sc-lint.toml

# Run every required source test gate through the consumer contract.
test: _source-build _source-venv
    .sc-lint/bootstrap test --config sc-lint.toml

# Inspect the managed upgrade path without changing the source checkout.
upgrade: _source-build _source-venv
    .sc-lint/bootstrap upgrade --config sc-lint.toml --check --dry-run
