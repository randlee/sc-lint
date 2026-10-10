# Anchors - phase-i

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-i/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | Four layers: shared types/attributes, cargo tools, CLI, just template | 2026-10-10 |
| ANC-002 | locked | Repo owns just test; sc-lint supplies patterns and helpers | 2026-10-10 |
| ANC-003 | locked | Each change lands in the lowest capable layer; sprint names it | 2026-10-10 |
| ANC-004 | locked | Redesign needs Rand's prior approval | 2026-10-10 |
| ANC-005 | locked | Only targeted additions required by #193-#204 | 2026-10-10 |
| ANC-006 | locked | Sprint designs carry code for every deliverable | 2026-10-10 |

## ANC-001

Anchor: Each change sits in the layer that owns it: (a) shared types and attributes, (b) independent cargo analyzer tools, (c) the sc-lint CLI, which calls (b), (d) the just template; no layer depends on a higher one and no (b) tool depends on another (b) tool.

> sc-lint must be a layered design.  w/ layers (lowest to highest):  a) sc-lint-types (shared types) and sc-lint-attribute (shared attributes)  b) independent cargo tools (i.e. sc-lint-boundary)  c) sc-lint cli which calls into (b)  d) just template (recommended just command layout w/ recommended mapping to (b) and (c) )

Said by Rand Lee, planning session, 2026-10-10. Status: locked. Version 3.

Summary: Four layers: shared types/attributes, cargo tools, CLI, just template

Scope: every phase-i change: architecture documentation, crate dependencies, CLI commands, and the adoption-kit just template

Deterministic check to pair with the development gate: cargo metadata --format-version 1 --no-deps: sc-lint-boundary, sc-lint-portability and sc-lint-runtime depend on no sc-lint-* crate outside sc-lint-schema, sc-lint-directives, sc-lint-analyzer-support, sc-lint-attributes; those (a) crates depend on no (b) or (c) crate; docs/architecture.md has a Layered Architecture section naming layers (a)-(d)

Beads: anchor `lint-hl1`, plan questions `lint-hl1.1`, dev questions `lint-hl1.2`

Fixtures:
- pass: Layer (b), sc-lint-portability. The analyzer reads its own table through the shared parser in sc-lint-analyzer-support; it gains no dependency on sc-lint-boundary or on the sc-lint CLI.
- pass: Layer (c), the sc-lint CLI. dispatch.rs forwards --rule-filter unchanged to the sc-portability backend; the CLI does not interpret selectors.
- pass: Layer (a), sc-lint-analyzer-support. Adds a TOML table parser that the three analyzers call; the crate gains no dependency on any analyzer or on the CLI.
- fail: Add sc-lint-boundary as a dependency of sc-lint-portability so portability can reuse the boundary inventory loader.
- fail: Move PORT-006 prefix matching into crates/sc-lint/src/dispatch.rs so the CLI filters portability findings itself.
- fail: sc-lint-schema now depends on the sc-lint CLI crate so the shared Finding type can carry the CLI's exit codes.

## ANC-002

Anchor: A consumer's `just test` stays defined and customized in the consumer repository; sc-lint ships only recipe patterns and general helper functions, never test orchestration.

> I expect 'just test' to be managed and customized by the repo.  sc-lint supplies patterns and general helper functions

Said by Rand Lee, planning session, 2026-10-10. Status: locked. Version 5.

Summary: Repo owns just test; sc-lint supplies patterns and helpers

Scope: the adoption-kit just template, any sc-lint CLI test command, and any helper sc-lint ships for consumer test recipes

Deterministic check to pair with the development gate: git diff of the phase adds no consumer test orchestration to crates/sc-lint; the kit's test recipe is a consumer-editable pattern; any sc-lint addition for testing is a general helper the recipe calls

Beads: anchor `lint-9lk`, plan questions `lint-9lk.1`, dev questions `lint-9lk.2`

Fixtures:
- pass: The kit's test recipe stays a plain just recipe that the consumer edits; sc-lint adds a general helper, `sc-lint util list-crates`, that the recipe may call.
- pass: The kit justfile shows a recommended `test *args:` recipe pattern; the consumer replaces its body with their own test commands.
- fail: Add a [test] manifest to sc-lint.toml with selectors and defaults; `sc-lint test <selector>` reads it and runs each step.
- fail: The CLI's workflow.rs gains step groups, parallel execution and keep_going for the consumer's test steps.
- fail: The kit test recipe becomes a managed block that the installer overwrites on every upgrade.

## ANC-003

Anchor: Each phase-i change lands in the lowest layer that can carry it, and each sprint bead names that layer.

> The intent is to make these changes at the appropriate layer

Said by Rand Lee, planning session; Draft wording by flint from Rand directive 2026-10-10; wording approved by Rand 2026-10-10; source words: "The intent is to make these changes at the appropriate layer", 2026-10-10. Status: locked. Version 3.

Summary: Each change lands in the lowest capable layer; sprint names it

Scope: every phase-i sprint bead and its delivered diff

Deterministic check to pair with the development gate: each sprint bead names one layer (a)-(d); git diff --stat of its PR touches only that layer's crates or kit files

Beads: anchor `lint-yn8`, plan questions `lint-yn8.1`, dev questions `lint-yn8.2`

## ANC-004

Anchor: No phase-i document or plan bead changes a layer's responsibilities, an ADR decision, or the crate set unless Rand approved that change in discussion first.

> any substantial redesign work is to be discussed w/ me before documented or planned.

Said by Rand Lee, planning session; Draft wording by flint from Rand directive 2026-10-10; wording approved by Rand 2026-10-10; source words: "any substantial redesign work is to be discussed w/ me before documented or planned.", 2026-10-10. Status: locked. Version 4.

Summary: Redesign needs Rand's prior approval

Scope: all phase-i documentation, ADR and plan-bead changes

Deterministic check to pair with the development gate: every diff to docs/architecture.md, docs/requirements.md or docs/sc-lint/adr/ on plan/phase-i, and every sprint bead that changes a layer responsibility, cites a recorded Rand approval

Beads: anchor `lint-zjw`, plan questions `lint-zjw.1`, dev questions `lint-zjw.2`

Fixtures:
- pass: Adds `analyze_workspace_configured` to sc-lint-runtime and a --config flag to its binary.
- pass: Adds the install key justfile.windows_shell to the kit schema and installer.
- pass: Amends ADR-012's canonical template to drop `default: lint`, as Rand approved in discussion on 2026-10-10.
- fail: Amend ADR-012 so `just test` is implemented by the CLI's test command.
- fail: Create a new crate, sc-lint-config, to hold analyzer configuration.
- fail: Move consumer step execution from the just template into crates/sc-lint/src/workflow.rs.

## ANC-005

Anchor: Every phase-i deliverable is a targeted addition that addresses a named GitHub issue from #193-#204; nothing is added that those issues do not require.

> I do NOT want unauthorized additions beyond what is required to address the issues filed. / These should all be line item targeted additions not redesign

Said by Rand Lee, planning session; Draft wording by flint from Rand directive 2026-10-10; wording approved by Rand 2026-10-10; source words: "I do NOT want unauthorized additions beyond what is required to address the issues filed. / These should all be line item targeted additions not redesign", 2026-10-10. Status: locked. Version 4.

Summary: Only targeted additions required by #193-#204

Scope: every phase-i sprint bead and deliverable

Deterministic check to pair with the development gate: every deliverable in every sprint bead cites one issue from #193-#204; each sprint PR diff is additive and confined to what that issue names

Beads: anchor `lint-7fs`, plan questions `lint-7fs.1`, dev questions `lint-7fs.2`

Fixtures:
- pass: Deliverable 1 (#203): owner_crate_path is checked against the lib target name from cargo metadata. Deliverable 2: a test for a package with a [lib] name.
- pass: Deliverable 1 (#200): `sc-lint lint <analyzer>` exits 10 on status fail. Deliverable 2: cli-contract.md lists exit code 10.
- pass: Deliverable 1 (#202): portability reads unix_path_prefixes from [tool.sc-lint.analyzers.portability], extending PORT-001 and PORT-006.
- fail: Deliverable 3: rename every RuleId variant to a new naming scheme for consistency.
- fail: Deliverable 4: add an approval matrix and a gate ledger to the root bead that track every ruling.
- fail: Deliverable 2: introduce a RuleCatalog trait registry so third-party analyzers can register at runtime.
- fail: Deliverable 5 (#200): add a --fail-on flag so users can choose which statuses fail.

## ANC-006

Anchor: Every sprint bead's design gives the code for each deliverable, so the developer makes no design decisions.

> I want the plans to be rich with code samples so that the agent doesn't need to make decisions when developing.

Said by Rand Lee, planning session; Draft wording by flint from Rand directive 2026-10-10; wording approved by Rand 2026-10-10; source words: "I want the plans to be rich with code samples so that the agent doesn't need to make decisions when developing.", 2026-10-10. Status: locked. Version 4.

Summary: Sprint designs carry code for every deliverable

Scope: every phase-i sprint bead's design field

Deterministic check to pair with the development gate: each sprint bead design contains a fenced code block for every numbered deliverable

Beads: anchor `lint-57u`, plan questions `lint-57u.1`, dev questions `lint-57u.2`

Fixtures:
- pass: Deliverable 1: exit 10 on a fail verdict. Design:
```rust
pub const EXIT_VERDICT_FAIL: u8 = 10;
```
- pass: Deliverable 1: the shared parser. Design:
```rust
pub fn load_analyzer_settings(catalog: &RuleCatalog, root: &Path) -> Result<AnalyzerSettings, AnalyzerConfigError> {
<redacted: 40 of 44 code lines are in the plan but were not sent to Jev>
```
- pass: Deliverable 1: drop the kit default recipe. Design: delete lines 1-4 of packages/sc-lint-adoption/.sc-lint/justfile.
- fail: Deliverable 1: add config parsing. Design: choose a suitable TOML library and decide the error type.
- fail: Deliverable 2: improve cycle precision. Design: the developer determines which edges to exclude.
- fail: Deliverable 1: handle Justfile collisions as appropriate; either warn or fail, whichever fits best.
