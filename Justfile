set windows-shell := ["cmd.exe", "/c"]

host_python := if os_family() == "windows" { "python" } else { "python3" }
# Source recipes run the sc_lint helper package from the repo-local venv (G.3a).
python_cmd := if os_family() == "windows" { ".\\.sc-lint\\venv\\Scripts\\python.exe" } else { ".sc-lint/venv/bin/python3" }
sc_lint_binary := if os_family() == "windows" { ".\\target\\debug\\sc-lint.exe" } else { "./target/debug/sc-lint" }
export SC_LINT_BIN := sc_lint_binary
adoption_binary := if os_family() == "windows" { "target/adoption-product/debug/sc-lint.exe" } else { "target/adoption-product/debug/sc-lint" }

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
_lint-function-length:
    {{python_cmd}} -m sc_lint.lint_function_length

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

[private]
_source-repo-pytests:
    {{python_cmd}} -m pytest -q bindings/sc-lint-py/python/sc_lint/tests/test_consumer_contract.py bindings/sc-lint-py/python/sc_lint/tests/test_consumer_lifecycle.py bindings/sc-lint-py/python/sc_lint/tests/test_orchestration_contracts.py bindings/sc-lint-py/python/sc_lint/tests/test_release_artifacts.py .claude/skills/closing-triage/scripts/test_query_open_findings.py .claude/skills/graph-orchestration/scripts/test_validate_findings.py .claude/skills/triaging-findings/tests/test_check_dependencies.py .claude/skills/triaging-findings/tests/test_triage_record_template.py .claude/skills/triaging-findings/tests/test_qa_triage_prompt.py .just/tests/test_prerelease_tag.py .just/tests/test_team_lead_roster_check.py .just/tests/test_lint_boundaries_rust_parity.py tests/adr tests/adoption/test_skill_assets.py tests/docs/test_action_docs.py

# Build the full workspace.
build:
    cargo build --workspace

# Build the product binary and the backend binaries it re-enters from
# `target/debug` (`full`/`ci` profiles dispatch sc-boundary/sc-portability
# through the running executable, so the siblings must exist beside it).
[private]
_source-build:
    cargo build --workspace

# Provision .sc-lint/venv with the sc_lint helper wheel built from this checkout.
[private]
_source-venv:
    {{host_python}} bindings/sc-lint-py/python/sc_lint/source_venv.py

# Run the complete source-maintainer test suite behind the consumer profile.
# Every suite runs even after one fails (and cargo runs every test binary), so one
# run reports every failure.
[private]
_source-test:
    cargo build --workspace
    {{python_cmd}} -m sc_lint.run_all "cargo test --workspace --no-fail-fast" "{{python_cmd}} -m sc_lint.run_pytests" "just _source-repo-pytests" "node --test action/test/action.test.cjs"

# Remove workspace build artifacts.
clean:
    cargo clean

# Run the complete source-maintainer lint suite behind the consumer profile.
[private]
_source-lint target='full':
    {{python_cmd}} -m sc_lint.run_lint {{target}}

# Verify the root model's compatible product binary and report setup state.
setup: _source-build _source-venv
    {{host_python}} .sc-lint/bootstrap.py setup --config sc-lint.toml --dry-run

# Run every required source lint gate through the consumer contract.
lint *args: _source-build _source-venv
    {{host_python}} .sc-lint/bootstrap.py lint --config sc-lint.toml {{if args == "ci" { "all" } else { args }}}

# Run every required source test gate through the consumer contract.
test: _source-build _source-venv
    {{host_python}} .sc-lint/bootstrap.py test --config sc-lint.toml

# Run the vendorable consumer-adoption kit integration suite. Requires pytest
# in the host Python: `python3 -m pip install 'pytest>=8,<9'` (POSIX) or
# `python -m pip install 'pytest>=8,<9'` (Windows).
test-adoption:
    {{host_python}} -m pytest tests/adoption -q

# Run the adoption tests that execute the kit's just layers against a product
# built into target/adoption-product, never the shared target/debug or
# .sc-lint/wheels. Missing artifacts fail the run.
test-adoption-integration:
    cargo build --workspace --target-dir target/adoption-product
    {{host_python}} -m pip wheel --quiet --disable-pip-version-check -w target/adoption-product/wheels bindings/sc-lint-py
    {{host_python}} -m pytest tests/adoption -q -m integration --integration --sc-lint-bin {{adoption_binary}} --sc-lint-wheel-dir target/adoption-product/wheels

# Inspect the managed upgrade path without changing the source checkout.
upgrade: _source-build _source-venv
    {{host_python}} .sc-lint/bootstrap.py upgrade --config sc-lint.toml --check --dry-run
