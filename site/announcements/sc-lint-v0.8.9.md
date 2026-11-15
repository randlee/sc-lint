# sc-lint v0.8.9 — Environment and Dependency Boundaries, a Panicking-Call Rule, and Implementation-Visibility Checks

**Release:** 0.8.9 (November 13, 2026) · **Also covered:** 0.8.8 (November 11, 2026) · **Install:** `brew install randlee/tap/sc-lint` (Homebrew), `pip install sc-lint` (PyPI), `cargo install sc-lint` (crates.io), or the platform archives on the [release page](https://github.com/randlee/sc-lint/releases/tag/v0.8.9)

[Changelog](https://github.com/randlee/sc-lint/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-lint/releases/tag/v0.8.9) · [Operator manual](https://github.com/randlee/sc-lint/blob/main/docs-bundle/README.md)

> **Upgrade straight to 0.8.9.** 0.8.9 shipped two days after 0.8.8 and carries the
> config-compatibility fix-ups that 0.8.8 introduced. Both releases are covered
> here because no 0.8.8 announcement was published on `main`; items are ordered
> by end-user impact and each is marked with the release that added it.

---

## Architecture-Boundary Enforcer — env reads, dependency requirements, honest visibility

**As a maintainer, I want sc-lint to enforce architecture boundaries for agents, so that agent-authored changes stay inside the intended module/crate boundaries instead of leaking across layers.**

**0.8.9** adds two opt-in policy families to the boundary analyzer, and both cover
things that previously had no rule at all. `SCB-ENV-001` (`env_var_boundary`)
reports literal `std::env::var`/`var_os` reads — including the `env::var` and
`env::var_os` spellings — outside declared reader functions, for workspace
packages under configured crate roots, so secrets and socket paths stop being
read wherever an agent happens to need them. `SCB-MANIFEST-007` and
`SCB-MANIFEST-008` (`dependency_requirements`) check manifests themselves: 007
reports exact and tilde registry requirements in template, test and packaged
manifests, and 008 reports an extra upper bound that narrows a caret-compatible
range unless the dependency line carries a matching `# upper-bound: <api|links|toolchain> ...`
comment. That turns "this template pinned the consumer to a version we do not
ship" into a lint finding instead of a downstream install failure.

**0.8.8** added `SCB-BOUNDARY-005`, which compares each boundary record's
declared `[implementation]` type and constructor visibility against the source
and fails when the source is wider than declared, or when the item is no longer
found. Boundary records stop being documentation that quietly drifts from the
code they describe. Repositories with an explicit `fail_rules` list must add the
rule ID to make it gating.

**Breaking in 0.8.8:** the boundary record's `[enforcement]` section
(`lint_rules`, `review_gates`) is removed. No check ever read it, so a record that
still declares it now fails to load with a diagnostic naming the file — delete
the section from every boundary record. `0.8.9` softens the surrounding
roughness: archive entries that escape the archive root are classified as a
configuration error (exit `3`) rather than an internal failure, and
`version_sync` reports `python.dependency_specifier` when a listed Python project
depends on another listed project with a specifier that does not admit the
workspace version — the stale-`atm-graft>=1.4,<1.5`-after-a-bump class of bug,
with `expected` and `actual` both in the finding.

## Agent Orchestrator — fleet-wide dependency and manifest conventions

**As an orchestrator, I want sc-lint to keep agent-authored code conformant to shared conventions, so that parallel agents' output stays consistent without per-agent review.**

`dependency_requirements` is the piece an orchestrator actually gets leverage
from. Point `template_manifests`, `test_manifests` and `package_archives` at the
globs your fleet generates, and the same rule set that lints workspace crates
also lints the manifests those crates hand to other repositories. Agents that
invent a tighter upper bound to make their own build green are caught in the PR
that introduced it, against the convention every SC repo shares, rather than
after a consumer adopts it.

Runtime rules now analyze supported Cargo example targets, including nested
modules, while `SCB-RUNTIME-003` and `004` keep their example exclusions
(REQ-RUNTIME-003). Examples are
where agents put the code they intend to copy into production, so analyzing them
closes one of the last blind spots in the fleet's lint coverage. Installer
backend probes in **0.8.8** now use the configured timeout, which removes a
platform-dependent hang from the setup path orchestrators run on every fresh
worktree.

## Agent Team Member — self-serviceable diagnostics

**As an agent on an SC team, I want excellent, actionable error messages, so that I can self-correct my code without a human pointing at the mistake.**

**0.8.9** adds `SCB-RUNTIME-008`, which rejects panicking calls in packages that
opt in through `[tool.sc-lint.analyzers.runtime] panic_rule_packages` — opt-in
because `unwrap`/`expect` in a startup path is a judgement call, not a universal
violation. Diagnostics stay pointed at the fix: a removed `[enforcement]` section
names the file to edit, an archive escape reports exit `3` as a configuration
error instead of an internal one, and `env_var_boundary` findings name the reader
function you should have called. All of it is available in `--json` for
programmatic consumption.

## Upgrade notes

- **Breaking:** delete the `[enforcement]` section from every boundary record (0.8.8). Records that keep it fail to load.
- `SCB-BOUNDARY-005` and the optional policy families are non-gating until listed in an explicit `fail_rules` list.
- The standalone `sc-lint-boundary` Homebrew formula is retired (#153). Install `randlee/tap/sc-lint` for the product **and** its analyzers — the analyzer binaries ship with it.
- `env_var_boundary` detection is syntactic: it does not expand macros, resolve cross-file aliases, evaluate computed variable names, or see `env::vars`. `dependency_requirements` validates the `upper-bound` comment's form and version, not the existence of the named break.

---

**What's next:** the boundary analyzer now owns environment and manifest policy
alongside structural rules; subsequent work continues tightening the runtime rule
set and the consumer qualification path that gates each release.
