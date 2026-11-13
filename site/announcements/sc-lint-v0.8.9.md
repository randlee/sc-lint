# sc-lint v0.8.9 — Panics in Opted-In Packages, Environment Boundaries, Dependency Requirements

**Released:** November 13, 2026 · **Install:** `brew install randlee/tap/sc-lint` (Homebrew), `pip install sc-lint` (PyPI), `cargo install sc-lint` (crates.io), or the platform archives on the [release page](https://github.com/randlee/sc-lint/releases/tag/v0.8.9)

[Changelog](https://github.com/randlee/sc-lint/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-lint/releases/tag/v0.8.9) · [Operator manual](https://github.com/randlee/sc-lint/blob/main/docs-bundle/README.md)

---

## Architecture-Boundary Enforcer

**As a maintainer, I want sc-lint to hold environment reads and dependency requirements to declared policy, so that a secret cannot be read from anywhere in the tree and a manifest cannot quietly pin what policy says it must not pin.**

Two policy families arrive in v0.8.9, both consumer-owned and both off until configured.

`env_var_boundary` enables `SCB-ENV-001`. It reports literal `std::env::var` and `var_os` reads — including the `env::var` and `env::var_os` spellings, and same-file forwarding helpers and wrappers — that happen outside the reader functions you name. Test code and variables you did not list are exempt.

```toml
[[tool.sc-lint.analyzers.boundary.env_var_boundary]]
forbidden_env_vars = ["APP_SECRET"]
restricted_crate_roots = ["crates/server"]
boundary_reader_functions = ["server::config::read_secret"]
```

Detection is syntactic by design: macros are not expanded, imported aliases and cross-file calls are not resolved, computed variable names are not evaluated, method calls are not followed, and other read APIs such as `env::vars` are not covered. That boundary is stated in the manual rather than left for you to discover from a miss.

`dependency_requirements` enables `SCB-MANIFEST-007` and `SCB-MANIFEST-008` over workspace manifests plus the template, test-fixture and package-archive paths you configure. `SCB-MANIFEST-007` reports an exact or tilde registry requirement — the pin that reads as `=1.5.0` and makes every consumer's upgrade an incident. `SCB-MANIFEST-008` reports an extra upper bound with no matching bound comment, so the bound has to carry the reason it exists.

```toml
[[tool.sc-lint.analyzers.boundary.dependency_requirements]]
template_manifests = ["templates/**/Cargo.toml"]
test_manifests = ["tests/fixtures/**/Cargo.toml"]
package_archives = ["target/package/*.crate"]
```

Both families follow the existing configuration contract: an unknown key, an unknown rule, or a wrong value type is a configuration error identifying the analyzer and the field, not a silently ignored line.

---

## Agent Orchestrator

**As an orchestrator, I want agent-authored code held to a no-panic policy in the packages that adopted one, so that a parallel agent cannot turn a `todo!()` into a production crash path.**

`SCB-RUNTIME-008` detects production `.unwrap()`, `.expect(...)`, `panic!`, `unreachable!`, `todo!` and `unimplemented!` — but only in the packages you name:

```toml
[tool.sc-lint.analyzers.runtime]
panic_rule_packages = ["worker"]
```

An absent or empty list opts in nothing, so upgrading changes no verdict for a repository that has not asked for this rule. Unknown, duplicate and empty package names are configuration errors. Test files and targets, inline `cfg(test)`, `#[test]` and framework test bodies are excluded, and existing analyzer rule and path filters still apply.

Matching is syntactic and independent of receiver type; macro bodies are not expanded and no type-aware proof is performed. That is the honest scope: the rule catches the call you can read in the source, not the one a macro generates.

---

## Agent Team Member

**As an agent on an SC team, I want configurations and archives to fail loudly and precisely, so that a malformed input names itself instead of producing a plausible wrong result.**

Runtime rules now analyze supported Cargo example targets, including examples that live in nested modules. Risk and panic rules 003 and 004 keep their existing example exclusions, so the widening is scoped to the rules that asked for it. If your examples carry lint debt that the library target does not, expect new findings on upgrade.

An archive entry that escapes the archive root is now a configuration error, exiting `3`. Subtly-wrong input that used to be read as if it were well-formed is now refused, with the same exit status every other configuration error uses — which is what lets a CI job distinguish "the tool could not run" from "the code failed a rule".

One removal, and it is a packaging change rather than a behavior change: the standalone `sc-lint-boundary` Homebrew formula is retired. Install `randlee/tap/sc-lint` for the product and its analyzers. The formula was also deleted from `randlee/homebrew-tap`, so a tap update removes it for anyone who installed it directly.

---

## Upgrade Notes

Nothing in this release changes a verdict for a repository that has not opted in: `env_var_boundary`, `dependency_requirements` and `panic_rule_packages` are all absent by default, and an empty `panic_rule_packages` list opts in no packages. The changes that can surface new findings on upgrade without any configuration are the runtime rules' widened example-target analysis and the stricter archive validation.

Repositories that keep an explicit `fail_rules` list must add `SCB-RUNTIME-008`, `SCB-ENV-001`, `SCB-MANIFEST-007` and `SCB-MANIFEST-008` to it before any of the new checks gate — each rule is reported but does not fail a run until listed, while a default-configured repository gains the gating immediately.

If you installed `sc-lint-boundary` from the tap, replace it with `randlee/tap/sc-lint`.

The Windows package is pending upstream (`winget-pkgs#450816`); crates.io, PyPI, Homebrew and scoop are published for v0.8.9.
