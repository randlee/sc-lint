# ADR-021 — Consumer Setup, Python Runner And Cfg Companions

| Field | Value |
| --- | --- |
| ID | ADR-021 |
| Status | Withdrawn (never accepted); superseded by [ADR-022](./ADR-022-tools-and-consumer-configuration.md) |
| Deciders | Rand |
| Related | ADR-010, ADR-016, ADR-017, ADR-020; #198, #720, #786 |

## Context

atm-core's [#198 answer](https://github.com/randlee/sc-lint/issues/198#issuecomment-6513016954)
commits to separate repository Python, kit Python and cargo/PyO3 execution.
Its 29 repository steps include three test steps; twelve OS-specific profile
entries invoke four wheel-only helpers. Cargo clippy also needs the repository
interpreter in `PYO3_PYTHON` and its executable directory on child `PATH`.
ADR-020 decisions 9/13 currently reject these settings. The accepted
restrictions remain effective until Rand accepts this proposal.

The [#786 shape table](https://github.com/randlee/sc-lint/issues/786#issuecomment-6513016641)
pins 47 findings in 18 paths at atm-core `6ffaa97be`. Seventeen paths are
classified false positives; `atm_temp.rs` mixes a false positive with three
deliberate POSIX-only helpers. S3 generalizes #720 beyond the crate root to
module functions and inherent methods. Rand ruled that S8 POSIX-only public
helpers are exempt because they are not a cross-platform parity gap.

## Proposed Decision

1. **Consumer setup.** Support `[tool.sc-lint.setup].pre` as one non-empty
   argv array. The committed consumer form is `pre = ["just", "bootstrap"]`.
   Bootstrap `setup` runs it once from the repository root before binary
   discovery, downloads, wheel creation or declared-tool provisioning. Parse
   and validate config first; execute argv without a shell. A nonzero child
   status stops setup and is returned unchanged. Spawn failure reports
   `CLI.SC_LINT_SETUP_PRE_FAILED`, command, cause, recovery and exit 4.
   Check and dry-run never execute it; dry-run displays it. Upgrade, lint,
   test and exec do not run it. The pre-wheel reader uses only the standard
   library; absent configuration preserves current setup behavior.
2. **Interpreter selection for lint and test.** Support
   `[tool.sc-lint.runner].python` as a non-empty consumer venv path;
   atm-core will use `python = ".bootstrap-venv"`. Resolve relative paths
   from repo root and accept absolute paths. Exactly a leading `python`
   selects `<venv>/bin/python` on Linux/macOS or
   `<venv>/Scripts/python.exe` on Windows. When unset, leading `python`
   selects the kit interpreter or its explicit consumer override in decision
   3. Preserve subsequent argv, test argument
   forwarding and scheduler dependencies. Do not rewrite `python3`, explicit
   executable paths or later `python` tokens. Selection applies equally to
   selected lint and test steps; list/skipped/unselected steps do not resolve
   interpreters.
3. **Kit helpers.** A command `sc-lint-python -m sc_lint.<module> [args...]`
   selects the pinned kit interpreter by default on every OS. Also recognize
   `python -m sc_lint.<module>` as a kit helper before applying the repository
   Python rule. `runner.python` never overrides these helper executables.
   A consumer may knowingly override that default with the non-empty venv
   path `[tool.sc-lint.runner].kit_python`. Resolve relative paths from repo
   root and accept absolute paths. This selects `<venv>/bin/python` or
   `<venv>/Scripts/python.exe` for both helper forms and the no-runner plain
   Python fallback. It is separate from `runner.python`; when both are set,
   repository script steps use `runner.python` and helpers use `kit_python`.
   Without the override, resolve `.sc-lint/venv/bin/python3` or
   `Scripts/python.exe`, matching bootstrap's wheel environment. A selected
   missing default or override fails; never fall back to PATH or source-tree
   modules. The consumer owns provisioning the override venv and its required
   `sc_lint` package; setup does not create or populate that environment.
   This replaces OS-specific paths without adding one command per helper.
   Existing source-maintainer adapters retain their separate source fallback.
4. **Child environment.** When `runner.python` is configured, prepend its
   bin/Scripts directory to each selected consumer step's child `PATH`,
   including cargo/clippy/xwin and kit-helper steps. Interpreter selection
   still follows decisions 2–3. Optional
   `runner.export_env = ["PYO3_PYTHON"]` exports the absolute repository
   interpreter path to those children. This initial list accepts only
   `PYO3_PYTHON`; an empty/absent list exports nothing. A non-empty list
   requires `runner.python`. Explicit runner exports replace inherited
   values; unrelated inherited environment is preserved. Use host path-list
   rules, preserve spaces, and never mutate the parent environment.
5. **S3 companions (#720).** A production `cfg(unix)` free function or
   inherent-impl method has PORT-010 parity with a non-test, same-kind,
   same-name sibling guarded by exactly `cfg(not(unix))` in the same lexical
   module or inherent impl. Apply at crate root, inline modules and file
   modules. Retain explicit Windows companions and portable fallbacks.
   Different names/scopes/impls, test-only siblings, `not(windows)` and
   unrelated cfgs do not establish S3 parity. Trait impls keep their existing
   treatment. This is syntactic matching, not general Boolean equivalence.
6. **Reviewed declarations (S2/S4/S7).** Add analyzer-owned
   `platform_companions` records, separate from unchanged file-pair
   `platform_adapters`. These three variants name `unix_file`, `unix_item`,
   `counterpart_file`, `counterpart_item`, `kind` and non-empty `reason`.
   Files must be confined existing repository-relative `.rs` files; items
   resolve uniquely in production AST scopes. Kinds are `function` (S2,
   differently named Windows/not-unix companion), `transport` (S4, unguarded
   portable transport), and `refusal` (S7, explicit non-unix config rejection).
   A refusal counterpart must contain an explicit non-unix error branch;
   standalone transport endpoints need no shared caller.
   Qualified item syntax and validation are specified in m-5b. Validate both
   declarations, applicable cfgs and production reachability; missing, stale,
   ambiguous or test-only declarations are config errors. Never infer
   semantic counterparts from spelling or co-location. A record covers its
   exact Unix item, not the whole file, unrelated public items or arbitrary
   callers. The reason records reviewed equivalence/refusal; the analyzer
   checks structure, not behavioral equivalence of platform implementations.
7. **Strict module boundaries.** A private `cfg(unix)` module alone is not
   an exemption. Preserve a PORT-010 finding on an unpaired module boundary
   unless a verified counterpart or approved, reasoned exact declaration
   covers the exact item. Building module/cfg context is analysis, not an
   exemption for the module or its descendants: m-5a retains S6 findings;
   m-5b may apply approved declarations to the exact items they name.
   No blanket file or private module opt-out is introduced. Rand's approved
   exception path is an exact
   reviewed declaration with a required reason; no counterpart means strict
   reporting unless that path explicitly applies.

Decisions 1–4 supersede ADR-020 decision 9 and amend decision 13's closed key
list only for nested `setup`/`runner`; root-level tables remain rejected.
They amend ADR-016 and REQ-CLI-021 only for these runtime settings; proposed
REQ-CLI-026 becomes effective on acceptance. Bootstrap/scaffolding stay
Python/declarative. Rust keeps profile planning, scheduling and runtime
executable/environment selection; no second Python planner is introduced.
Decisions 5–7 extend ADR-010/ADR-017 structural parity only as stated.

## Decided Rand Ruling — S8 Exempt

`atm_temp.rs::is_owned_by_uid` and `current_uid` are public POSIX-only
helpers without Windows counterparts. Rand ruled on 2026-10-30 that these
are exempt: the POSIX-only concepts are not a parity gap. Represent this
through an exact reviewed `platform_companions` record with
`kind = "posix_only"`, `unix_file`, `unix_item` and non-empty `reason`.
This variant needs no counterpart fields; validate its confined production
item and Unix cfg. This is the approved, reasoned opt-out path Rand requested,
not automatic suppression of public POSIX-only items. It does not exempt
other public helpers or
hard-code atm-core names in product defaults. The third deliberate helper,
`has_group_or_world_bits`, is private and is evaluated under S1.

Final acceptance is zero PORT-010 findings on all 18 paths with exclusions
removed, while true gaps in negative fixtures remain reported. The S8 scope
ruling is settled and its separate human gate is resolved. ADR-021 as a whole
remains Pending, including this declaration design; implementation remains
blocked on its acceptance. Every S8 record requires a reason.

## Errors And Approval

Malformed fields, unknown nested keys, unsupported exports and invalid module
marker argv fail as `CLI.CONFIG_ERROR` (exit 3), naming config path, field,
cause, recovery and `sc-lint docs configuration`. Missing selected repository
or kit interpreters fail before spawn with respectively
`CLI.SC_LINT_RUNNER_PYTHON_UNAVAILABLE` or
`CLI.SC_LINT_KIT_PYTHON_UNAVAILABLE`, path, recovery, `sc-lint docs setup`,
exit 4. Rust codes are typed constants in their owning module. Companion
config errors use the existing analyzer-config envelope with indexed field,
cause and recovery; rule IDs and finding JSON stay stable.

ADR-021 remains Pending. Its human gate blocks m-1, m-3 and m-5b until Rand
accepts the applicable decisions and activates the CLI amendment on the
planning branch. Rand also confirmed that `runner.export_env` with
`PYO3_PYTHON` and the venv on child `PATH` is in m-3 scope; this scope ruling
does not accept ADR-021 as a whole. m-5a's conservative
S1/S5/S6 work does not depend on S8 or declarations; it consumes m-1's S3
proof. Draft anchors are not approval. No post-hook DSL, automatic consumer
venv creation or new analyzer dependency is proposed.
