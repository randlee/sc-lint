# ADR-022 — Tools And Consumer Configuration, Not Orchestration

| Field | Value |
| --- | --- |
| ID | ADR-022 |
| Status | Base ADR accepted — Rand, 2026-10-31 ("3 - accept"); #822/#784 amendment Proposed, Pending Rand acceptance |
| Deciders | Rand |
| Related | ADR-010, ADR-012, ADR-015, ADR-016, ADR-017, ADR-020, ADR-021 (withdrawn); REQ-CLI-021, REQ-CLI-023, REQ-CLI-028 (Proposed, Pending); REQ-PRODUCT-023, REQ-SCB-019; #136, #198, #720, #786, #822, #784 |

## Context

Rand (2026-10-30): "I think a lot of sc-lint issues come from attempts to
mandate how the sc-lint tools are orchestrated where the focus should be aimed
at creation of tools that provide consistent/flexible consumer configuration."

The atm-core P1 issues show the pattern:

- **#198.** atm-core asked for setup hooks and interpreter selection. Its
  2026-10-30 evidence shows the glue is "mostly convenience; nothing is
  strictly impossible on 0.8.2 today". The one hard case is `PYO3_PYTHON` on a
  clippy lint step. Lint steps reject `env`, although ADR-016 already promises
  "per-step/per-layer `env`" and test selectors accept it.
- **#136.** The open finding is a tool defect: SCB-MANIFEST-004 treats
  `self::cron::`, `super::cron::` and `crate::cron::` as imports of the `cron`
  crate. Owner `tests/` targets are expressed today by per-entry
  `allowed_source_roots`.
- **#786/#720.** PORT-010 misses two common companion forms (a same-name
  `#[cfg(not(unix))]` sibling and a `#[cfg(unix)] mod x;` file). For the
  remaining shapes and for deliberate POSIX-only helpers, sc-lint has no
  narrow per-site allowance outside the boundary analyzer.

Meanwhile REQ-CLI-021, ADR-012 and ADR-016 fix entry points, recipe shapes
and interpreter choice. That is orchestration, and it is where most
consumer friction arises.

## Decision

1. **Product scope.** sc-lint provides analyzers, commands and a consistent
   consumer configuration surface. The consumer owns orchestration: entry
   points, recipe and CI shape, ordering of tools, interpreter and environment
   choice, and setup steps. sc-lint never requires a particular orchestration
   to obtain a result.

2. **Configuration surfaces.** These are the consumer's configuration
   surfaces. Each has one job:

   | Surface | Controls |
   | --- | --- |
   | `sc-lint.toml` `[tool.sc-lint]` | version gate, lint steps and profiles, test selectors |
   | `[tool.sc-lint.analyzers.<name>]` | rule enablement, severity, rule and path exclusions, analyzer-specific policy |
   | Boundary inventory TOML (`boundaries/*.toml`) | boundary records, callers, contracts, dependency and manifest policy |
   | Source attributes `#[sc_lint(...)]` | per-item policy at the site it governs |
   | `.sc-lint/install.json` | exact external tool versions and targets |
   | CLI flags | per-invocation selection (`--config`, `--profile`, `--platform-scope`, `--rule-filter`, `--fail-fast`, `--list`) |
   | Environment variables | binary and install location only (`SC_LINT_BIN`, `SC_LINT_INSTALL_DIR`, `SC_LINT_RELEASE_BASE_URL`) |

   A new consumer setting goes on one of these surfaces.

3. **Parallel keys.** A lint step and a test command selector accept the same
   execution keys: `command`, `env`, `env_paths` (proposed by amendment
   decision 4), `cwd`, platform restriction and `after_all`.

4. **PORT-010 site allowance.** PORT-010 can be allowed at a single site with
   `#[sc_lint(allow(PORT-010), reason = "...")]`. This is the exception path
   for a deliberate POSIX-only item or a companion shape PORT-010 does not
   recognise. The report lists each honoured allowance with its reason. There
   is no repository-wide switch that disables a rule without naming paths or
   sites.

5. **Managed files are templates, not requirements.** `.sc-lint/bootstrap.py`,
   the managed `Justfile` block and the generated `sc-lint.toml` are a
   convenience starting point. A consumer may replace or delete them. Every
   tool is reachable through the `sc-lint` command with an explicit
   `--config`, with the same results.

6. **No orchestration hooks in the product.** sc-lint does not add setup
   hooks or interpreter-selection policy. These are consumer orchestration.
   A consumer expresses them with step argv, step `env` and its own
   recipes.

## Effect On Existing Decisions

- **ADR-016 "How To Extend".** Unchanged. Decision 3 enforces its
  "per-step/per-layer `env`" promise on lint steps.
- **ADR-020 decision 9.** The outcome stands: `setup.pre` and `runner.python`
  are not added. The reason becomes decision 6 here.
- **ADR-021 (Withdrawn).** Superseded and withdrawn. atm-core withdrew the need
  for its setup and runner keys. Its PORT-010 companion shapes move to the
  ADR-010 amendment "PORT-010 companion shapes" (Phase M).
- **ADR-010 amendment (2026-10-12).** It says "narrow, explicit adapter
  configuration, not a general repo-configured suppression". Decision 4 here
  is consistent with it: a site allowance is narrow and explicit.
- **ADR-012 and REQ-CLI-021.** The required recipe shapes, the `python` /
  `python3` choice and bootstrap-only entry become properties of the
  managed template (decision 5), not consumer obligations.
- **Directive grammar** (`docs/sc-lint-directives/requirements.md`). `allow`
  is extended from the boundary scope to PORT-010 (decision 4). The existing
  grammar stays backward compatible.

## Consequences

- #198 reduces to adding `env` and `cwd` to lint steps.
- #136 reduces to fixing the local-module false positive in SCB-MANIFEST-004
  and documenting owner `tests/` roots in `allowed_source_roots`.
- #786 shapes PORT-010 does not recognise, including S8, have a defined
  exception path.

## Proposed Amendment — Pending Rand Acceptance (#822, #784)

This amendment is a proposal only. The base ADR remains Accepted at its
existing scope; the following amendment remains Pending until Rand accepts it
in his own words. The proposed requirement and full worked example are in
[REQ-CLI-028](../cli-requirements.md#req-cli-028--proposed-pending-rand-acceptance).

1. **Decision: host selection applies to consumer lint rows only.** Add an
   optional host selector, independent of target platform, so a consumer can
   configure host-specific command environments. Test selectors do not accept
   this field in the amendment.
2. **Decision: add a lint-only bare-run profile default.** Add optional
   `default_lint_profile` so consumers choose a named lint profile for bare
   lint while retaining explicit CI profile selection. It does not apply to
   `sc-lint test`.
3. **Rationale for #822 and #784.** Default `host` scope already silently removes
   `platform = "windows"` xwin rows on Linux and macOS. It does not remove a
   Windows-host Windows-platform CI row from a Windows developer's bare run.
   `default_lint_profile` allows CI to select that row while bare lint selects
   the local profile. The example keeps native Windows clippy in `local` and
   uses a distinct CI-only explicit-target `--all-targets` row, so the latter
   adds target-wide CI coverage rather than duplicating default-target clippy.
   Host-specific consumer environments also keep interpreter choice in the
   consumer configuration, consistent with the base ADR's ownership boundary.
4. **Decision: config-root-relative path environment.** Add optional
   `env_paths` to lint steps and test selectors. It maps environment keys to
   paths: relative values resolve against the directory containing the loaded
   config file, while absolute values pass through unchanged. Do not
   canonicalize paths, resolve symlinks, check existence, or prepend to `PATH`.
   Keep `env` literal. Build scripts such as PyO3's may run with the package
   directory as their cwd, so a repo-relative value in `env` resolves from the
   wrong directory. `env_paths` lets the consumer declare the path explicitly
   without sc-lint knowing a venv placeholder or layout.

The detailed semantics, validation, compatibility, report examples and
acceptance criteria are owned by REQ-CLI-028.

Alternatives considered:

- `env_by_host`: rejected because it adds host branching inside environment
  configuration instead of keeping each command and its literal environment
  together in a selectable row.
- Venv placeholder: rejected because it makes sc-lint interpret a
  consumer-specific filesystem convention. `env_paths` instead carries an
  explicit consumer-declared path and gives sc-lint no knowledge of venv
  layout.
- Unix alias: rejected because Linux and macOS are distinct host values with
  different interpreter paths and execution support.
- Per-step `default = false`: rejected because it adds a second inclusion
  rule to each row and complicates the meaning of explicit `all`; the
  consumer-level named profile default keeps inclusion in profile membership.
