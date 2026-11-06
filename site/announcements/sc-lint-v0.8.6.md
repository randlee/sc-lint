# sc-lint v0.8.6 — Runtime Guardrails, Consumer-Owned Templates, Library Target Scanning

**Release:** 0.8.6 · **Supersedes:** 0.8.5 (November 4, 2026 — see the note below) · **Install:** `brew install randlee/tap/sc-lint` (Homebrew), `pip install sc-lint` (PyPI), `cargo install sc-lint` (crates.io), or the platform archives on the [release page](https://github.com/randlee/sc-lint/releases/tag/v0.8.6)

[Changelog](https://github.com/randlee/sc-lint/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-lint/releases/tag/v0.8.6) · [Operator manual](https://github.com/randlee/sc-lint/blob/main/docs-bundle/README.md)

> **Upgrade straight to 0.8.6.** 0.8.5 shipped one day earlier and is affected by
> defects that 0.8.6 fixes: a dangling-symlink adoption seed failure, a
> `sc-lint init --just` refresh path that could not pick up an unedited 0.8.4
> integration, and a Python 3.9 lint path that could not start. Both releases are
> covered here because 0.8.5 never had an announcement of its own; items are
> ordered by end-user impact, with the release that introduced each one marked.

---

## Architecture-Boundary Enforcer — new runtime rules, honest library-target graphs

**As a maintainer, I want sc-lint to enforce architecture boundaries for agents, so that agent-authored changes stay inside the intended module/crate boundaries instead of leaking across layers.**

The largest change in this pair is a set of runtime rules that catch the blocking
patterns agents reach for when they are trying to get async code to work. In
**0.8.5**, `SCB-RUNTIME-004` through `007` reject, in production targets of
packages that declare Tokio: thread `sleep_until`, Tokio timer sleeps,
Runtime/Handle/futures `block_on`, and std `Mutex` guards held across `.await`.
`SCB-RUNTIME-003`'s recovery text was rewritten to recommend event-driven
synchronization or `yield_now`, because `tokio::time::sleep(..).await` is now
itself a finding under `SCB-RUNTIME-005`. Odd as it sounds, agent output quality
went *up* when the advice stopped pointing at another blocking sleep. Test code
stays exempt: an out-of-line `#[cfg(test)] mod checks;` file now counts as test
scope for all seven rules if it carries `#![cfg(test)]` at the top, and one
shared test-scope classifier handles `#[test]`, `#[tokio::test]`, `#[cfg(test)]`
and compound cfgs such as `#[cfg(all(test, ...))]`. Import alias resolution
(`self::`, `super::`, `crate::`, globs, renames) is now per module and function
scope, so results no longer depend on item order in a file.

Boundary analysis was also fixed where it was quietly blind. In **0.8.6**, graph
export and source discovery cover **all** Cargo library target kinds — proc
macros and multi-kind libraries included — instead of only plain `lib` targets.
A crate that exposes its API through a `proc-macro` target was previously
invisible to the boundary graph, which is exactly the shape of crate an agent
will happily reach across a layer for. The shared supported-target predicate now
lives in one place rather than being restated per tool. Separately, the boundary
documentation points at the portability and runtime tools for rules that were
moved out of the boundary analyzer (#138), so `Docs:` pointers in diagnostics no
longer lead to a section that no longer owns the rule.

Release qualification got stricter too. **0.8.6** requires a native-verified
prerelease before release preflight and publish; native verification produces a
report for the release gate (#863). Portability site-allowance diagnostics were
also consolidated in **0.8.5** into `DIRECTIVE.SITE_ALLOW.*` codes, reporting
every finding across all scanned files in one sorted failure instead of only the
first — a deliberate behaviour break, described under Upgrade Notes.

## Agent Orchestrator — one profile set, consumer-owned templates, cross-platform CI

**As an orchestrator, I want sc-lint to keep agent-authored code conformant to shared conventions, so that parallel agents' output stays consistent without per-agent review.**

The template story changed direction in **0.8.6** and it is the item most likely
to affect a fleet-wide rollout. `sc-lint init` (and `init --just`) now treats the
generated `Justfile` and `sc-lint.toml` as **consumer-owned templates** and only
`.sc-lint/bootstrap.py` as product-owned (ADR-022, #893). Consequently `init`
requires `--just`, generated files carry a managed-by header, and `init --just`
compares after normalizing line endings so a CRLF checkout on Windows reads as
current or version-only stale instead of a user-owned conflict. Adoption seeds
**missing destinations only** and leaves existing consumer files alone (#851,
#862, #878), so re-running adoption against a repo with local edits no longer
overwrites work. The `init` output changed with it: human output reports
`init: <status>` plus `Created:` / `Refreshed:` / `Preserved:` /
`Version-only stale:` lists, and JSON adds a `refreshed_files` array told apart
from the preserved set by `status` and `dry_run`.

For CI, **0.8.6** adds a `tools.non_windows` declaration for cargo tools and Rust
targets that should only be installed on non-Windows hosts (#865) — the missing
half of cross-platform setup, since a Windows job previously had to choose
between failing on a Unix-only tool and hand-editing the workflow. Generated CI
now installs `rustfmt` and `clippy` on the **effective** Rust toolchain, so
consumers can delete that step from their own workflow (#885). The Python floor
moved down to **3.9** in **0.8.5** (`requires-python = ">=3.9"`, matching the
abi3-py39 wheels) and **0.8.6** closes the loop with Python 3.9-floor lint tests
run on supported hosts in CI (#894, #897). Windows ARM64 wheels still require
3.11.

Two narrower orchestrator-facing changes: the GitHub Action writes each failure
as a `::error` annotation titled with its stable code and the recovery text in
the message (**0.8.5**), which is what makes an annotation readable by an agent
picking work back up from a failed run; and the Action runs on Node 24 with six
supported runners listed in its description. Consumers adopting the kit keep
`skills/` and `agents/` at the plugin root, where Claude Code's loader finds
them, while the same files still install under `.claude/` in consumers.

## Agent Team Member — diagnostics you can self-correct from, help that exits 0

**As an agent on an SC team, I want excellent, actionable error messages, so that I can self-correct my code without a human pointing at the mistake.**

**0.8.5** made the entire command surface self-describing: every `sc-lint`
command, option and target value has help text, `-V` is a short form of
`--version`, usage errors show the usage of the *failing* subcommand and suggest
`sc-lint <subcommand> --help`, and compatibility errors give specific recovery
paths (a missing config suggests `sc-lint init --just`; a malformed config asks
you to correct the reported error; a nonexistent `--root` says so). An unknown
`--rule-filter` selector now lists the valid selectors for that analyzer.
Generated CLI references for all four binaries ship in the offline bundle in
Markdown and HTML, and `sc-lint docs cli-reference` prints the index — tests
regenerate them from the real clap definitions and fail on drift, so the docs
cannot silently fall behind the parser.

**0.8.6** removes the most common agent stumble of all: running `sc-lint` with no
arguments now prints help and exits **0** for every binary, while invalid
arguments still fail (exit 2). An agent probing a bare binary gets an answer
instead of a spurious failure it has to reason about. Allowances are stricter and
clearer at the same time: an `#[sc_lint(allow(...), reason = "...")]` on an item
that is not a function or method is an error instead of being silently ignored,
and an allowance naming several unknown `SCB-RUNTIME-*` ids reports every one,
not just the first (lint-085.46). `sc-lint docs` also accepts `README`,
`README.md`, `overview` and `setup` (as an alias of `installation`), so the
`Docs: sc-lint docs setup` pointer in setup and compatibility errors resolves
instead of sending you to a dead guide name.

Python consumers get the same treatment. `python -m sc_lint.run_all`,
`run_fmt` and `source_venv` answer `--help`, `print_help` lists only recipes that
actually exist in the `Justfile`, and the adoption installer's `--help` describes
`--input`, `--dry-run`, the persisted input and its exit codes.

---

## Upgrade Notes

- **Python 3.9 and older checkouts.** The `tomllib` fallback needs Python 3.11 or
  newer; a source checkout that has not run `just setup` yet therefore needs
  either 3.11+ or the built native extension (which parses TOML through
  `sc_lint._native.toml_loads`). With neither, parsing raises `ImportError` with a
  recovery message. (**0.8.5**)
- **Behaviour-breaking — site-allowance codes.** Every site-allowance failure is
  now `DIRECTIVE.SITE_ALLOW.*`: an unsupported `PORT-*` id other than `PORT-010`
  reports `DIRECTIVE.SITE_ALLOW.UNKNOWN_RULE`, `PORTABILITY.UNKNOWN_RULE` and
  `PORTABILITY.INVALID_SITE_ALLOW` are gone, and the exit code stays 3. Tooling
  that matches the old codes must match the new ones. (**0.8.5**)
- **Behaviour-breaking — Python backend exit codes.** A Python helper that fails
  to start is now a backend failure (exit 5) carrying stderr and suggesting
  `just setup`; it was exit 6. Scripts that test exit 6 from `sc-lint view
  findings` or another Python-backed command must also accept 5. (**0.8.5**)
- **`sc-lint init` requires `--just`**, and its human output is no longer
  `init: ok`. Parse `status`, `created_files`, `refreshed_files`,
  `preserved_files` and `version_only_stale_files` from JSON instead of the prose
  line. (**0.8.5** changed the shape, **0.8.6** changed ownership semantics.)
- **Bootstrap usage.** `setup` accepts only `--dry-run`; `--check` belongs to
  `upgrade`. `setup --check` is a bootstrap usage error now, not a forwarded
  command. (**0.8.5**)
- **Rust API.** `PortabilityError::UnknownRule` and `InvalidSiteAllow` are
  removed in favour of `InvalidSiteAllows(SiteAllowErrors)`; `PackageName` and
  `TargetName` no longer implement `Deref`; `count_scanned_crates` is gone. Use
  `as_str()` and match `InvalidSiteAllows`. CLI and Python users need no
  configuration migration. (**0.8.5**)
- **Runtime rules will flag existing code.** Blocking sleeps, timer sleeps,
  `block_on` and `Mutex` guards held across `.await` in production Tokio
  packages are findings from `SCB-RUNTIME-003` through `007`. Migrate to
  event-driven synchronization or `yield_now`, or use a reason-required
  per-function allowance. (**0.8.5**)
