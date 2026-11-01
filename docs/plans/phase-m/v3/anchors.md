# Anchors - phase-m

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | retired | P1a | 2026-10-30 |
| ANC-002 | retired | P2a | 2026-10-30 |
| ANC-003 | locked | P3e | 2026-10-30 |
| ANC-004 | retired | P4 | 2026-10-30 |
| ANC-005 | retired | P5 | 2026-10-30 |
| ANC-006 | locked | P6d | 2026-10-30 |
| ANC-007 | locked | P7b | 2026-10-30 |
| ANC-008 | retired | P8c | 2026-10-30 |
| ANC-009 | locked | P9 | 2026-10-30 |
| ANC-010 | locked | P10c | 2026-10-30 |
| ANC-011 | locked | P11 | 2026-10-30 |
| ANC-012 | locked | P12 | 2026-10-30 |
| ANC-013 | locked | P13f | 2026-10-30 |

## ANC-001

Anchor: sc-lint's PORT-010 companion-parity check must count a private or pub(crate) #[cfg(unix)] helper as having a companion, and not report it, when every production reference to it is a call inside a #[cfg(unix)] item that itself has a companion PORT-010 recognises: a #[cfg(windows)] or #[cfg(not(unix))] sibling, a renamed non-Unix counterpart, an un-gated portable alternative, or status as such a helper.

> REQ-PRODUCT-004AA/PORT-010 (docs/requirements.md): "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"

ADR-010 decision 6 (Accepted): "Explicit `#[cfg(unix)]` production boundaries are treated as structural parity questions for the shared portability family. Leaf rules may suppress inside those items, with the companion-or-fallback decision owned by the structural parity rule family instead of parallel per-rule allowlists."

Issue #786 (atm-core verdict table, 2026-10-30): "Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S1  Private (or pub(crate)) cfg(unix) helper whose only callers are cfg(unix) items that themselves have a Windows/not(unix) sibling. Rows 1, 2, 3, 5, 6, 7, 10, 11."

Said by REQ-PRODUCT-004AA PORT-010 + ADR-010 decision 6 + #786, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P1a, 2026-10-30. Status: retired. Version 5.

Summary: P1a

Scope: The item changes how PORT-010 treats a private or pub(crate) #[cfg(unix)] helper whose production callers are #[cfg(unix)] items that have a companion (#786 shape S1, including helpers of sibling, renamed and loopback-TCP pairs and helper chains), in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 stops reporting a private #[cfg(unix)] helper whose only callers are #[cfg(unix)] functions that have #[cfg(windows)] siblings. Evidence: test port010_s1_helper_of_paired_fn passes, showing zero findings for that fixture.
- pass: PORT-010 stops reporting a private #[cfg(unix)] helper whose only caller is a #[cfg(unix)] transport function with an un-gated loopback TCP alternative. Evidence: test port010_s1_helper_of_s4_pair passes, showing zero findings.
- fail: PORT-010 still reports a private #[cfg(unix)] helper even though its only caller has a #[cfg(windows)] sibling.
- fail: PORT-010 exempts a helper only when its caller has a same-name or #[cfg(windows)] sibling; a helper whose caller has a renamed or loopback TCP counterpart is still reported.

## ANC-002

Anchor: sc-lint's PORT-010 companion-parity check must count a differently named #[cfg(windows)] or #[cfg(not(unix))] function, such as connect_named_pipe for connect_unix, as the companion of a #[cfg(unix)] function, and not report the #[cfg(unix)] function, when both have the same return type, each is called in production only from the matching gated branch of one caller, and neither is called anywhere else.

> REQ-PRODUCT-004AA/PORT-010 (docs/requirements.md): "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"

ADR-010 decision 6 (Accepted): "Explicit `#[cfg(unix)]` production boundaries are treated as structural parity questions for the shared portability family. Leaf rules may suppress inside those items, with the companion-or-fallback decision owned by the structural parity rule family instead of parallel per-rule allowlists."

Issue #786 (atm-core verdict table, 2026-10-30): "Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S2  cfg(unix) fn whose cfg(windows) companion has a different name (`connect_unix` / `connect_named_pipe`, `unix_default_scratch_root` / `windows_default_scratch_root`, `unix_synthesized_..._path` / `windows_synthesized_..._path`). Rows 1, 5, 9."

Said by REQ-PRODUCT-004AA PORT-010 + ADR-010 decision 6 + #786, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P2a, 2026-10-30. Status: retired. Version 5.

Summary: P2a

Scope: The item changes how PORT-010 pairs a #[cfg(unix)] function with a differently named #[cfg(windows)] function (#786 shape S2), in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 accepts a differently named #[cfg(windows)] companion such as connect_named_pipe for connect_unix. Evidence: test port010_s2_renamed_companion passes, showing zero findings for that pair.
- fail: PORT-010 still reports connect_unix although connect_named_pipe is its #[cfg(windows)] companion.

## ANC-003

Anchor: sc-lint's PORT-010 rule (which reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) must not report a #[cfg(unix)] item that has a #[cfg(not(unix))] sibling of the same name and kind in the same scope, whether that scope is a module, an inherent impl or a crate root (lib.rs, main.rs or a src/bin file).

> Issue #786 (atm-core verdict table, 2026-10-30): "Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S3  cfg(unix) item with a same-name `#[cfg(not(unix))]` sibling in the same scope (module, inherent impl, or crate root). Rows 2, 4, 6, 14, 18. This is #720 generalized beyond crate-root main.rs."

REQ-PRODUCT-004AA (docs/requirements.md), rule list: "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"

Said by REQ-PRODUCT-004AA PORT-010 + ADR-010 decision 6 + #786, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P3e, 2026-10-30. Status: locked. Version 4.

Summary: P3e

Scope: The item changes how PORT-010 treats a #[cfg(unix)] item with a same-name #[cfg(not(unix))] sibling in the same scope (#720, #786 shape S3), in crates/sc-lint-portability.

Beads: anchor `None`, plan questions `None`, dev questions `None`

Fixtures:
- pass: PORT-010 stops reporting a #[cfg(unix)] item with a same-name #[cfg(not(unix))] sibling in a module, an inherent impl and the crate root. Evidence: test port010_s3_same_name_sibling passes, showing zero findings for all three scopes.
- fail: PORT-010 exempts the crate-root main.rs pair only; module and inherent-impl same-name siblings are still reported.

## ANC-004

Anchor: sc-lint's PORT-010 rule must not report a #[cfg(unix)] transport or adapter function whose every production reference is in a #[cfg(unix)] branch or match arm of one caller whose non-Unix branch or un-gated match arm uses an un-gated loopback TCP function or enum variant instead, which Windows uses.

> REQ-PRODUCT-004AA/PORT-010 (docs/requirements.md): "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"

ADR-010 decision 6 (Accepted): "Explicit `#[cfg(unix)]` production boundaries are treated as structural parity questions for the shared portability family. Leaf rules may suppress inside those items, with the companion-or-fallback decision owned by the structural parity rule family instead of parallel per-rule allowlists."

Issue #786 (atm-core verdict table, 2026-10-30): "Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S4  cfg(unix) transport/adapter fn or match arm beside an unguarded loopback-TCP sibling (enum variant or fn) that serves Windows. Rows 8, 11, 12, 15."

Said by REQ-PRODUCT-004AA PORT-010 + ADR-010 decision 6 + #786, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P4, 2026-10-30. Status: retired. Version 5.

Summary: P4

Scope: The item changes how PORT-010 treats a #[cfg(unix)] transport or adapter function used only from a Unix branch or match arm beside an un-gated loopback TCP alternative (#786 shape S4), in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 stops reporting a #[cfg(unix)] Unix-socket connect function called only from the #[cfg(unix)] arm of a match whose un-gated TcpLoopback arm serves Windows. Evidence: test port010_s4_loopback_tcp_sibling passes, showing zero findings for that fixture.
- fail: PORT-010 still reports the #[cfg(unix)] Unix-socket connect function even though the caller's un-gated TcpLoopback arm serves Windows.

## ANC-005

Anchor: sc-lint's PORT-010 rule (which reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) must not report a #[cfg(unix)] helper of any visibility whose every production reference is inside a #[cfg(unix)] block, statement or expression of a function whose body also has a #[cfg(not(unix))] block or statement.

> REQ-PRODUCT-004AA/PORT-010 (docs/requirements.md): "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"

ADR-010 decision 6 (Accepted): "Explicit `#[cfg(unix)]` production boundaries are treated as structural parity questions for the shared portability family. Leaf rules may suppress inside those items, with the companion-or-fallback decision owned by the structural parity rule family instead of parallel per-rule allowlists."

Issue #786 (atm-core verdict table, 2026-10-30): "Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S5  cfg(unix) helper used only inside a cfg(unix) block of a function whose cfg(not(unix)) block is in the same body. Rows 10, 16."

Said by REQ-PRODUCT-004AA PORT-010 + ADR-010 decision 6 + #786, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P5, 2026-10-30. Status: retired. Version 5.

Summary: P5

Scope: The item changes how PORT-010 treats a #[cfg(unix)] helper used inside a #[cfg(unix)] block of a function that also has a #[cfg(not(unix))] block (#786 shape S5), in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 stops reporting a #[cfg(unix)] helper used only inside a #[cfg(unix)] block of a function whose #[cfg(not(unix))] block is in the same body. Evidence: test port010_s5_cfg_blocks_in_one_fn passes, showing zero findings for that fixture.
- fail: PORT-010 still reports the helper called from the #[cfg(unix)] block although the same function has a #[cfg(not(unix))] block.

## ANC-006

Anchor: sc-lint's PORT-010 rule (which reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) must not report the declaration #[cfg(unix)] mod x; of a module that is never compiled on Windows.

> Issue #786 (atm-core verdict table, 2026-10-30): "Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S6  Module declared `#[cfg(unix)] mod x;` and items inside it also carry cfg(unix); module is never compiled on Windows. Rows 13 (the decl), 17."

REQ-PRODUCT-004AA (docs/requirements.md), rule list: "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"

Said by REQ-PRODUCT-004AA PORT-010 + ADR-010 decision 6 + #786, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P6d, 2026-10-30. Status: locked. Version 3.

Summary: P6d

Scope: The item changes how PORT-010 treats a #[cfg(unix)] mod x; declaration (#786 shape S6), in crates/sc-lint-portability.

Beads: anchor `None`, plan questions `None`, dev questions `None`

Fixtures:
- pass: PORT-010 no longer reports a #[cfg(unix)] mod unix_socket; declaration. Evidence: test port010_s6_unix_module_decl passes, showing zero findings for the declaration line.
- fail: PORT-010 still reports the #[cfg(unix)] mod unix_socket; declaration line.

## ANC-007

Anchor: sc-lint's PORT-010 rule must recognize, and not report, items carrying #[cfg(unix)] inside a module declared #[cfg(unix)] mod x;, since that module is never compiled on Windows.

> REQ-PRODUCT-004AA (docs/requirements.md), rule list: "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"

Issue #786 (atm-core verdict table, 2026-10-30): "Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S6  Module declared `#[cfg(unix)] mod x;` and items inside it also carry cfg(unix); module is never compiled on Windows. Rows 13 (the decl), 17."

Said by REQ-PRODUCT-004AA PORT-010 + ADR-010 decision 6 + #786, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P7b, 2026-10-30. Status: locked. Version 3.

Summary: P7b

Scope: The item changes how PORT-010 treats #[cfg(unix)] items inside a module declared #[cfg(unix)] mod x; (#786 shape S6), in crates/sc-lint-portability.

Beads: anchor `None`, plan questions `None`, dev questions `None`

Fixtures:
- pass: PORT-010 no longer reports #[cfg(unix)] items inside a module declared #[cfg(unix)] mod unix_socket;. Evidence: test port010_s6_items_in_unix_module passes, showing zero findings inside that module.
- fail: PORT-010 still reports each #[cfg(unix)] function inside the #[cfg(unix)] mod unix_socket module.

## ANC-008

Anchor: sc-lint's PORT-010 companion-parity check must count a #[cfg(not(unix))] function or block in the same scope that takes a cross-platform config type and returns an error on every path as the portable fallback for a #[cfg(unix)] accessor or consumer of that type, and not report the accessor or consumer.

> REQ-PRODUCT-004AA (docs/requirements.md), rule list: "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"

Issue #786 (atm-core verdict table, 2026-10-30): "Result: **no TRUE POSITIVE found**. All 18 paths have a Windows (or portable) counterpart, or are unix-only by design. [...]

## Distinct shapes (what a rule must recognize)

- S7  cfg(unix) accessor or consumer of a cross-platform config type where non-unix deliberately rejects the config with a typed error. Rows 13 (`get`), 15."

Said by REQ-PRODUCT-004AA PORT-010 + ADR-010 decision 6 + #786, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P8c, 2026-10-30. Status: retired. Version 5.

Summary: P8c

Scope: The item changes how PORT-010 treats a #[cfg(unix)] accessor or consumer of a cross-platform config type that non-Unix code rejects with a typed error (#786 shape S7), in crates/sc-lint-portability.

Fixtures:
- pass: PORT-010 stops reporting a #[cfg(unix)] accessor on a cross-platform config type that non-Unix validation rejects with a typed error. Evidence: test port010_s7_rejected_config_accessor passes, showing zero findings for that fixture.
- fail: PORT-010 still reports the #[cfg(unix)] get() accessor although validate_config rejects that config on non-Unix with a typed error.

## ANC-009

Anchor: sc-lint's PORT-010 (sc-lint's rule that reports a production #[cfg(unix)] item lacking a #[cfg(windows)] companion or portable fallback) changes must not blanket-suppress real parity gaps: a production #[cfg(unix)] item with no #[cfg(windows)] companion and no portable fallback must still be reported unless that item carries its own reasoned #[sc_lint(allow(PORT-010), reason = "...")] (ANC-010).

> REQ-PRODUCT-004AA/PORT-010 (docs/requirements.md): "`PORT-010` - production structural `#[cfg(unix)]` companion-parity checks"

ADR-010 decision 6 (Accepted): "Explicit `#[cfg(unix)]` production boundaries are treated as structural parity questions for the shared portability family. Leaf rules may suppress inside those items, with the companion-or-fallback decision owned by the structural parity rule family instead of parallel per-rule allowlists."

Issue #786 (atm-core verdict table, 2026-10-30): "#720 covers only the crate-root main.rs shape; this issue tracks the complete remaining inventory and asks for per-case classification, not blanket suppression of real parity gaps."

Said by REQ-PRODUCT-004AA PORT-010 + ADR-010 decision 6 + #786, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P9, 2026-10-30. Status: locked. Version 3.

Summary: P9

Scope: The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item.

Beads: anchor `None`, plan questions `None`, dev questions `None`

Fixtures:
- pass: PORT-010 keeps reporting a production #[cfg(unix)] function with no #[cfg(windows)] companion and no portable fallback. Evidence: test port010_real_gap_still_reported passes, showing one finding for that fixture.
- fail: PORT-010 now skips every #[cfg(unix)] item in a file once any Windows code exists anywhere in that file.

## ANC-010

Anchor: PORT-010, a rule sc-lint's portability analyzer reports from source, must be allowable at a single site: an item carrying #[sc_lint(allow(PORT-010), reason = "...")] is not reported by PORT-010.

> ADR-022 (Rand 2026-10-30: "This is a positive step.  This should guide both anchors and decisions about how atm-core fixes are handled.  Proceed"), decision 4: "Every rule an analyzer reports from source can be allowed at a single site with `#[sc_lint(allow(<RULE>), reason = \"...\")]`. A site allowance is the approved exception path for a deliberate, non-parity case such as #786 S8."

Said by ADR-022 decision 4 + #786 S8 + Rand S8 ruling, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P10c, 2026-10-30. Status: locked. Version 3.

Summary: P10c

Scope: The item adds or changes per-site allowance of PORT-010 findings through #[sc_lint(allow(...))], in crates/sc-lint-portability, crates/sc-lint-directives or crates/sc-lint-attributes.

Beads: anchor `None`, plan questions `None`, dev questions `None`

Fixtures:
- pass: A pub #[cfg(unix)] fn current_uid annotated #[sc_lint(allow(PORT-010), reason = "POSIX uid has no Windows concept")] produces no PORT-010 finding. Evidence: test port010_site_allow passes, showing zero findings for the annotated item and one finding for an identical unannotated item.
- fail: PORT-010 still reports a pub #[cfg(unix)] fn current_uid even though it carries #[sc_lint(allow(PORT-010), reason = "POSIX uid has no Windows concept")].

## ANC-011

Anchor: A [[tool.sc-lint.lint]] step in sc-lint.toml must accept an env table and a cwd key, applied to that step's child process, with the same meaning they have on a test command selector.

> ADR-016 (Accepted), How To Extend: "Use declarative `sc-lint.toml` lint profiles, test layers, per-analyzer tables, step argv/exclusions, platform conditionals and per-step/per-layer `env`."

ADR-022 (Rand 2026-10-30: "This is a positive step.  This should guide both anchors and decisions about how atm-core fixes are handled.  Proceed"), decision 3.2: "Parallel constructs accept parallel keys. A lint step and a test command selector accept the same execution keys: `command`, `env`, `cwd`, platform restriction and `after_all`."

Said by ADR-016 How To Extend + ADR-022 decision 3.2 + #198, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P11, 2026-10-30. Status: locked. Version 3.

Summary: P11

Scope: The item changes which keys [[tool.sc-lint.lint]] steps accept or how lint steps launch their child process in crates/sc-lint.

Beads: anchor `None`, plan questions `None`, dev questions `None`

Fixtures:
- pass: A lint step with env = { PYO3_PYTHON = "/opt/atm/venv/bin/python" } and cwd = "crates/x" runs its command with that variable set and that working directory. Evidence: test lint_step_env_and_cwd passes, showing the child observed both.
- fail: sc-lint still rejects env on a [[tool.sc-lint.lint]] step with 'Lint steps accept only name, command, platform, target and after_all'.

## ANC-012

Anchor: sc-lint must not add a setup-hook or interpreter-selection configuration key, such as [tool.sc-lint].setup.pre or [tool.sc-lint].runner.python.

> ADR-022 (Rand 2026-10-30: "This is a positive step.  This should guide both anchors and decisions about how atm-core fixes are handled.  Proceed"), decision 6: "sc-lint does not add setup hooks or interpreter-selection policy. These are consumer orchestration. A consumer expresses them with step argv, step `env` and its own recipes."

ADR-020 decision 9 (Accepted): "**#198 (setup hook).** Won't-do. ADR-016's hook restriction is unchanged; `setup.pre` and `runner.python` are not added."

Said by ADR-022 decision 6 + ADR-020 decision 9, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P12, 2026-10-30. Status: locked. Version 2.

Summary: P12

Scope: The item adds or changes consumer setup, hook or interpreter-selection configuration keys in sc-lint.toml.

Beads: anchor `None`, plan questions `None`, dev questions `None`

Fixtures:
- pass: Lint steps gain env and cwd; no setup or runner key is added. Evidence: config test rejects [tool.sc-lint].setup and [tool.sc-lint].runner as unknown keys.
- fail: Add [tool.sc-lint.runner].python so a leading 'python' in step argv resolves to the consumer venv interpreter.

## ANC-013

Anchor: sc-lint's SCB-MANIFEST-004 check must not report a self::X, crate::X or super::X path, in a use declaration or anywhere else in the code, as a use of the owned crate X, nor a bare X:: path in a module that declares its own mod X.

> Issue #136 (atm-core, 2026-10-30): "`mod.rs:11` is `pub mod cron;` (file `commands/cron.rs`), and `crates/atm/Cargo.toml` has no `cron` dependency, so nothing imports the cron crate: it is **not a true violation and should not be reported**. [...] in a scratch file with no external dependency, 0.8.2 flags `use self::cron::X;`, `use crate::cron::X;` and `use super::cron::X;`, i.e. it matches the first path segment textually and does not distinguish a local module from the extern crate."

REQ-SCB-019 (docs/sc-lint-boundary/requirements.md): "Manifest findings use `SCB-MANIFEST-003` for a disallowed configured dependency, `-004` for a configured ownership violation, [...]"

Said by REQ-SCB-019 + #136 atm-core answer 5, Phase M v3: requirement/ADR-based derivation under ADR-022 direction, 2026-10-30; candidate P13f, 2026-10-30. Status: locked. Version 3.

Summary: P13f

Scope: The item changes how SCB-MANIFEST-004 (global_dependency_ownership) decides that a source path uses an owned dependency, in crates/sc-lint-boundary.

Beads: anchor `None`, plan questions `None`, dev questions `None`

Fixtures:
- pass: SCB-MANIFEST-004 skips use self::cron::X, use super::cron::X and use crate::cron::X in a non-owner crate with a local mod cron, while still reporting use cron::Schedule. Evidence: test manifest004_local_module_path passes.
- fail: SCB-MANIFEST-004 still reports crates/atm/src/commands/mod.rs for Cron(self::cron::CronCommand), where cron is a local module.
