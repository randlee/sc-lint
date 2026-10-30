# Planning gate r1: bead-text repairs (lint-09d2)

The six live sprint containers were revised after r1 (03bf1d60, report committed at 7a67df4f). This records the exact bead-text delta; Beads remains authoritative. Anchors, questions, fixtures, ADR status and requirements are unchanged. This is not a judge rerun or a planning-gate PASS.

## Authority and changes

| Bead | Repair | Existing authority |
| --- | --- | --- |
| m-1 | Unqualified S3 behavior across lexical scopes; approval still gates implementation; state six-item layer share | ANC-026, ANC-014; ADR-021 decision 5 |
| m-2 | Replace retired/draft traces | ANC-025, ANC-029 |
| m-3 | Explicit just bootstrap creation of .bootstrap-venv; repository-script/clippy vs kit-helper split; active traces | ANC-023/024/019/022/027/028/032/033; ADR-021 decisions 1–4 |
| m-4 | Replace retired/draft traces | ANC-016/030/031 |
| m-5a | State S1/S5 share and inherited S3 preservation | ANC-014/026; existing m-1 prerequisite |
| m-5b | Preserve S3 in final acceptance; identify #786 table precisely | ANC-014/026; existing final 47-item closure |

No new implementation surface, exception, gate or default was introduced. The m-3 helper exception is explicit rather than applying repository interpreter selection to kit helpers. Existing test-only/true-gap negatives and whole-ADR approval holds remain in force. No ADR/REQ amendment was needed for these bead-local omissions.

## Scope proposals for the lead

lint-09d2 scope triage (proposals only; no anchor/question/fixture changes):
- ANC-027 falsely reaches m-2, m-4, m-5b. Proposed scope: "The item implements or changes setup.pre argv parsing, validation or execution in consumer setup/bootstrap/installer flows." Generic consumer config parsing or a temporary reproduction config alone is not setup.pre work.
- ANC-031 falsely reaches m-2, m-3, m-5b. Proposed scope: "The item owns the three-key released-analyzer parity audit's gap-issue search/filing and links from docs/plans/phase-m/manifest-policy-parity.md." Merely deferring to the audit or maintaining the separate #786 classification table does not own this work.
- ANC-025 and ANC-029 falsely reach m-4. Proposed scope for both: "The item implements or changes source-discovery behavior or source_scope parsing/validation for global_dependency_ownership / SCB-MANIFEST-004 in sc-lint-boundary." An evidence-only audit of a published release is outside this implementation scope.
These are the eight r1 scope cells. Request Rand's amend-path decision; scopes remain locked and unchanged. Plan-text repairs are complete; validation is running. A gate rerun remains necessary and no PASS is claimed.

## Exact live-bead changes

```diff
--- lint-m-1/description/before
+++ lint-m-1/description/after
@@ -1,11 +1,11 @@
 ## Goal
 #720 + #786 S3: recognize cfg(unix)/cfg(not(unix)) same-name companions in crate-root, module and inherent-impl scopes. Pending ADR-021 decision 5 requires Rand approval before implementation.
 ## Deliverables
-1. Extend PORT-010's free-function AND inherent-method companion predicates to exact cfg(not(unix)) production siblings in the same lexical scope, retaining REQ-PRODUCT-002 finding shape and existing Windows/fallback behavior.
+1. Extend PORT-010's free-function AND inherent-method companion predicates to same-name cfg(not(unix)) siblings in the same lexical scope, retaining REQ-PRODUCT-002 finding shape and existing Windows/fallback behavior.
 2. Replace port010_fixture_crate_root_main_current_behaviour and widen regressions to all S3 source shapes in atm_temp_sweeper.rs::from_metadata (inherent method), persistence.rs::sync_parent_directory, atm-daemon-bootstrap/lib.rs::unix_socket_config, loopback_tcp.rs::{create_private_record_file,validate_record_parent}, and atm/src/main.rs::restore_default_sigpipe. Cover crate roots, inline modules and out-of-line modules. Add true-gap negatives: absent/different-name/different-scope/different-impl/test-only/unrelated-cfg sibling.
 3. Update PORT-010 docs and fixture provenance; expose the S3 companion classification to the subsequent conservative-helper pass in m-5a.
 ## Does Not Close
 m-5a/m-5b own S1/S2/S4-S8 and final #786 closure. A helper of an S3 paired function is not automatically exempt in m-1. Trait-impl behavior and general Boolean cfg equivalence remain unchanged.
 
 ## Active Trace
-GH-786.S3; ANC-014, ANC-018, ANC-020.
+ANC-026; ANC-018, ANC-020. ANC-014 layer share: the six S3 occurrences produce no PORT-010 finding; final 47-item closure belongs to m-5b.
--- lint-m-1/acceptance_criteria/before
+++ lint-m-1/acceptance_criteria/after
@@ -1,5 +1,5 @@
 ## Acceptance Criteria
-- issue:#720 and issue:#786 S3: after ADR-021 approval, all six named S3 item occurrences across five paths produce no PORT-010 finding, including from_metadata within its inherent impl. Tests with same names in another impl/module and true missing companions still report.
+- issue:#720 and issue:#786 S3: A cfg(unix) free function or inherent method with a same-name cfg(not(unix)) sibling in the same module, inherent impl or crate root produces no PORT-010 finding. This rule covers all six named S3 item occurrences across five paths, including from_metadata within its inherent impl. ADR approval gates implementation, not the promised S3 behavior. Tests with same names in another impl/module and true missing companions still report.
 - req:REQ-PRODUCT-002: existing Windows/fallback/adapter behavior, trait-impl checks, rule IDs and JSON remain stable; docs no longer claim the S3 limitation is crate-root-only.
 ## Validation
 - cargo test -p sc-lint-portability --no-fail-fast
--- lint-m-2/description/before
+++ lint-m-2/description/after
@@ -1,5 +1,5 @@
 ## Goal
-#136 / draft ANC-025 / GH-136.src-only.2: provide the released source-only equivalent atm-core still needs, without broadening ownership allowlists or excludes.
+#136 / ANC-025 / ANC-029: provide the released source-only equivalent atm-core still needs, without broadening ownership allowlists or excludes.
 ## Deliverables
 1. Add optional source_scope = "src" to each global_dependency_ownership entry. Omitted or "all" retains today's repository-wide discovery. Scope filters source scanning only: manifest dependency ownership remains enforced unchanged (REQ-PRODUCT-002, ADR-020 decision 4).
 2. Deterministically reproduce imports in separate crates/*/tests targets and production src files with identical allowed_source_roots. With src scope, tests targets are outside the scan, allowed owner source passes, and disallowed production src imports still report SCB-MANIFEST-004. Verify default behavior still reports test imports and malformed scope fails with an indexed field error.
@@ -8,4 +8,4 @@
 The three-key parity audit belongs to m-4. Do not suppress the reported crates/atm/src/commands/mod.rs cron import merely because it appears in consumer evidence: it is production src and remains a real ownership violation under the requested source-only scan. No AST import-parser rewrite or consumer config edits.
 
 ## Active Trace
-ANC-025; GH-136.src-only.2.
+ANC-025, ANC-029.
--- lint-m-3/description/before
+++ lint-m-3/description/after
@@ -2,12 +2,12 @@
 #198: replace atm-core's repository-Python, kit-helper and PyO3/clippy glue with supported consumer runtime config. Rand confirmed on 2026-10-30 that runner.export_env (PYO3_PYTHON plus venv on child PATH) is in scope; this does not accept ADR-021 as a whole. Blocked on Rand accepting revised ADR-021 decisions 1-4 and activating REQ-CLI-026.
 ## Deliverables
 1. Accept tool.sc-lint.setup.pre argv, runner.python and runner.kit_python venv paths and runner.export_env list. The consumer commitment is pre=["just","bootstrap"], python=".bootstrap-venv", export_env=["PYO3_PYTHON"]. Validate fields/types/empties and export allowlist; retain root-level table rejection (REQ-CLI-026).
-2. Run pre once from repo root before bootstrap provisioning, with ADR-021 status/failure and check/dry-run behavior. Synchronize all three bootstrap copies; direct binary setup must not run it again.
+2. Run pre=["just","bootstrap"] once from repo root: the consumer bootstrap builds .bootstrap-venv before sc-lint provisioning, replacing atm-core's setup glue, with ADR-021 status/failure and check/dry-run behavior. Synchronize all three bootstrap copies; direct binary setup must not run it again.
 3. Route selected lint AND TEST steps through one runtime resolver: plain python uses runner.python, falling back to kit Python when unset; sc-lint-python -m sc_lint.<module> and python -m sc_lint.<module> use kit Python by default; explicit runner.kit_python selects a consumer-provisioned override venv. runner.python alone never overrides helper choice. Missing selected kit/repo environments fail rather than silently using PATH. Preserve argv, test forwarding, scheduler edges, skipped steps and inspection-only test --list.
 4. Prepend configured runner bin/Scripts to every selected step's child PATH, including native cargo clippy, cargo xwin clippy and kit helpers; export PYO3_PYTHON when opted in. Explicit exports replace inherited values, preserve other environment and never modify parent environment. Executable choice for kit helpers follows kit default or explicit runner.kit_python regardless of child PATH.
-5. Add deterministic resolver/process/adoption tests, and migration docs for 26 lint + 3 test repository steps, four kit helpers with twelve OS-specific entries, and both native/xwin clippy steps. Show replacement of _py, python_cmd, _sc-lint-py, sc_lint_clippy_prefix and _sc-lint-clippy without embedded per-OS paths.
+5. Add deterministic resolver/process/adoption tests, and migration docs for 26 lint + 3 test repository steps, four kit helpers with twelve OS-specific entries, and both native/xwin clippy steps. Show replacement of _py, python_cmd, _sc-lint-py, sc_lint_clippy_prefix and _sc-lint-clippy without embedded per-OS paths. Repository-script steps formerly invoked by _py use runner.python=".bootstrap-venv"; native/xwin clippy recipe steps formerly wrapped by sc_lint_clippy_prefix and _sc-lint-clippy receive that same .bootstrap-venv interpreter through child PATH and PYO3_PYTHON. Any sc_lint.* helper invocation formerly reached through those recipes follows the separate kit default (ANC-022) or explicit kit_python override (ANC-019), not runner.python executable selection.
 ## Does Not Close
 No consumer repo edits, automatic creation of its venv, arbitrary environment assignment DSL, post hooks or built-in command per helper. All three interpreter needs A/B/C in the consumer answer are in scope; do not claim #198 fixed while any remains unsupported.
 
 ## Active Trace
-ANC-019, ANC-022, ANC-023, ANC-024; GH-198.setup-pre.2, GH-198.runner-argv.2, GH-198.runner-path.2, GH-198.export-env.3.
+ANC-019, ANC-022, ANC-023, ANC-024; ANC-027, ANC-033, ANC-028, ANC-032.
--- lint-m-3/acceptance_criteria/before
+++ lint-m-3/acceptance_criteria/after
@@ -1,7 +1,7 @@
 ## Acceptance Criteria
-- req:REQ-CLI-026: fresh just setup runs [just,bootstrap] once before provisioning; failure stops later operations and preserves child status; --check/--dry-run and non-setup commands never execute pre.
+- req:REQ-CLI-026: fresh just setup runs pre=["just","bootstrap"] once to build .bootstrap-venv before sc-lint provisioning; failure stops later operations and preserves child status; --check/--dry-run and non-setup commands never execute pre.
 - req:REQ-CLI-023/026: configured plain python in both lint and all three consumer test-step shapes resolves to repo venv; absent runner selects kit; marker and legacy python -m sc_lint.* select kit by default even when runner differs; explicit kit_python selects the override for both helper forms and the absent-runner plain-python fallback. Test default, runner-only, override-only and both-set combinations on all three host inputs. Explicit paths/python3/later tokens unchanged; spaces and test forwarded args intact.
-- issue:#198 C: both cargo clippy and cargo xwin clippy children and nested children observe runner bin/Scripts first on PATH and opted-in absolute PYO3_PYTHON; conflicting inherited values are overridden only in child env, parent/unrelated vars remain unchanged. Kit-helper executable follows its default/explicit override selection while receiving the declared child env.
+- issue:#198 C: both cargo clippy and cargo xwin clippy children replacing the _py/clippy recipe glue and nested children observe runner bin/Scripts first on PATH and opted-in absolute PYO3_PYTHON; conflicting inherited values are overridden only in child env, parent/unrelated vars remain unchanged. Kit-helper executable follows its default/explicit override selection while receiving the declared child env.
 - req:REQ-PRODUCT-002: malformed config/unsupported export/invalid marker is exit3 with field/context; missing selected repo/kit interpreter is exit4 with stable typed codes/recovery. Unselected/skipped/list paths need no interpreter. Existing execution order/failure policy/envelopes unchanged.
 - req:REQ-CLI-022: identical managed assets, init check/dry-run nonmutation, isolated adoption setup->lint AND setup->test plus four wheel-helper modules succeed with distinct kit/repo envs; migration examples remove all A/B/C glue without per-OS interpreter paths.
 ## Validation
--- lint-m-4/description/before
+++ lint-m-4/description/after
@@ -1,5 +1,5 @@
 ## Goal
-#136 / draft ANC-016 / GH-136.audit-verify / GH-136.audit-gaps: verify which of atm-core's three original boundaries policy keys the released analyzer covers, and file remaining config-surface gaps.
+#136 / ANC-016 / ANC-030 / ANC-031: verify which of atm-core's three original boundaries policy keys the released analyzer covers, and file remaining config-surface gaps.
 ## Deliverables
 1. Write docs/plans/phase-m/manifest-policy-parity.md with one row each for manifest_dependency_allowlists, global_dependency_ownership and manifest_section_rules. At audit start, select the latest PUBLISHED sc-lint release (0.8.2 at planning time), then pin its tag and commit SHA and the atm-core config/checker commit. For each key, map consumer fields to released fields/rules/entry points, name semantic differences and cite executable fixture evidence.
 2. During sprint execution, search existing GH issues and file only genuinely uncovered config-surface gaps, linked from the table; link #136 for the known source-scope issue rather than duplicating it. Covered keys must have an explicit evidence-based conclusion, including whether atm-core intentionally retains an extension.
@@ -7,4 +7,4 @@
 No feature implementation, consumer config/root/exclude changes or claim of full parity based on key names. m-2 owns source-only implementation. Do not wait for m-2: audit the pinned published release and distinguish planned changes from shipped capability.
 
 ## Active Trace
-ANC-016; GH-136.audit-verify, GH-136.audit-gaps.
+ANC-016, ANC-030, ANC-031.
--- lint-m-4/acceptance_criteria/before
+++ lint-m-4/acceptance_criteria/after
@@ -6,4 +6,4 @@
 - Verify every pinned source link, field/rule/CLI spelling and referenced fixture result; record commands/results in the report.
 - just lint
 - just test
-- GH-136.audit-verify: report proves the latest PUBLISHED release was selected at audit start and records its tag/SHA; no arbitrary older release or moving branch baseline. All three keys use that same immutable release.
+- ANC-030: report proves the latest PUBLISHED release was selected at audit start and records its tag/SHA; no arbitrary older release or moving branch baseline. All three keys use that same immutable release.
--- lint-m-5a/description/before
+++ lint-m-5a/description/after
@@ -9,4 +9,4 @@
 S2/S4/S7 reviewed declarations, S6 exact-item exceptions and decided S8 public-helper exemption are m-5b; final zero/18-path acceptance belongs there. No name heuristics, consumer default paths, blanket exclusion or widening allowed roots.
 
 ## Active Trace
-ANC-014, ANC-018, ANC-020.
+ANC-018, ANC-020. ANC-014 layer share: remove the proven S1/S5 false positives and report the exact residuals; m-5b owns final zero. ANC-026 preservation: inherited m-1 S3 same-name cfg(not(unix)) pairs remain unreported in module, inherent-impl and crate-root scopes; retain those regressions while adding caller-index fixtures.
--- lint-m-5a/acceptance_criteria/before
+++ lint-m-5a/acceptance_criteria/after
@@ -1,6 +1,6 @@
 ## Acceptance Criteria
 - issue:#786: S1/S5 helpers with fully proven paired/guarded callers stop reporting; adding one ungated/ambiguous/reference escape causes a finding again. S6 module/cfg facts never cause automatic suppression: unpaired private modules and unapproved items retain findings, independent of visibility or Unix-gated ancestors.
-- Full deterministic 18-path/47-item table reproduces 0.8.2 baseline, assigns every finding a shape and owner, and reports exact residual counts honestly. Reduced fixtures retain evidence-bearing bodies/module topology; unpaired/public true-gap fixtures still report.
+- Inherited m-1 S3 pairs remain free of PORT-010 findings across module, inherent-impl and crate-root scopes. Full deterministic 18-path/47-item table reproduces 0.8.2 baseline, assigns every finding a shape and owner, and reports exact residual counts honestly. Reduced fixtures retain evidence-bearing bodies/module topology; unpaired/public true-gap fixtures still report.
 - req:REQ-PRODUCT-002: RuleId/JSON stability; unresolved cases fail closed, no blanket path/name exemptions. S8 public helpers remain reported at this layer until m-5b adds their exact reviewed posix_only records.
 ## Validation
 - cargo test -p sc-lint-portability --no-fail-fast
--- lint-m-5b/description/before
+++ lint-m-5b/description/after
@@ -4,9 +4,9 @@
 1. Implement only the Accepted ADR-021 platform_companions surface: confined Unix/counterpart file refs, uniquely resolved qualified item refs, kind=function|transport|refusal and mandatory reason. Validate production cfg context and counterpart evidence; do not infer intention from names or adjacent code. Feed validated roots into the existing helper proof so their private helpers can qualify.
 2. Cover S2 differently named Windows companions; S4 portable loopback transport selected beside Unix arm/adapter; S7 accessor/config consumer with explicit non-unix refusal; and S6 only through verified counterpart evidence or approved reasoned exact declarations. Private cfg(unix) module status alone never grants an exemption, nor does one module declaration blanket-cover unapproved descendants. Add exact positive fixtures and missing/stale/ambiguous/test-only/wrong-cfg/unsupported-item negatives. Unrelated same-file Unix items keep reporting.
 3. Implement the decided S8 exemption for public POSIX-only is_owned_by_uid/current_uid using kind=posix_only with unix_file, unix_item and mandatory reason, without counterpart fields. Validate confined existing production Unix items and exact identity. Never hard-code consumer names in product defaults or make arbitrary public helpers inherit S1. ADR-021 declaration syntax remains Pending until whole-record acceptance.
-4. Verify all 18 paths with their PORT-010 exclusions removed using a temporary consumer config at pinned 6ffaa97be. Provide required reviewed declarations/reasons in the fixture/migration example, assert zero PORT-010 findings with the decided S8 exemption, and keep true-gap negative fixtures reported. Update docs/table and mark #720/#786 closure coverage honestly.
+4. Verify all 18 paths with their PORT-010 exclusions removed using a temporary consumer config at pinned 6ffaa97be. Provide required reviewed declarations/reasons in the fixture/migration example, assert zero PORT-010 findings with the decided S8 exemption, and keep true-gap negative fixtures reported. Update the #786 47-item classification/closure table and its fixture docs and mark #720/#786 closure coverage honestly.
 ## Does Not Close
 No Windows behavioral-equivalence claim: consumer investigators only confirmed some sibling existence, not their behavior. No consumer repo edits, generic Boolean cfg solver, default per-repo item list or blanket production suppression.
 
 ## Active Trace
-ANC-014, ANC-018, ANC-020, ANC-021.
+ANC-014, ANC-018, ANC-020, ANC-021; ANC-026 preservation from m-1: S3 pairs remain unreported in final verification.
--- lint-m-5b/acceptance_criteria/before
+++ lint-m-5b/acceptance_criteria/after
@@ -1,7 +1,7 @@
 ## Acceptance Criteria
 - issue:#786: every S2/S4/S7 exemption resolves exact production items plus applicable cfg/refusal evidence and nonempty reason; omitted/stale/malformed/ambiguous declarations error or keep the true gap visible. Same-file or similarly named unrelated items never become exempt.
 - S8 follows Rand's EXEMPT ruling: exact reviewed posix_only records with required reasons produce no PORT-010 findings for is_owned_by_uid/current_uid; unrelated public true-gap fixtures still report. Whole-ADR approval remains required for declaration syntax. Final accepted contract supports zero findings on all 18 pinned paths with PORT-010 exclusions removed.
-- Final 47-item table and full consumer reproduction show zero PORT-010 findings, including atm_temp.rs mixed cases; synthetic true parity gaps still report. Every configured relation/exception has a reason. Source pin and commands/results recorded in fixture provenance/PR evidence.
+- Final 47-item table and full consumer reproduction show zero PORT-010 findings, including atm_temp.rs mixed cases and all six inherited S3 same-name cfg(not(unix)) pairs in module, inherent-impl and crate-root scopes; synthetic true parity gaps still report. Every configured relation/exception has a reason. Source pin and commands/results recorded in fixture provenance/PR evidence.
 - req:REQ-PRODUCT-002: finding IDs/schema and existing adapters stable; unknown keys/kinds, non-confined paths, test-only/missing/cfg-invalid/ambiguous refs and missing reasons have deterministic typed errors with indexed fields/recovery. Consumer-neutral shipped defaults contain no atm-core-specific paths/items.
 ## Validation
 - cargo test -p sc-lint-portability --no-fail-fast

```

## Validation

Passed:

- `validate-plan --phase m --index docs/plans/phase-m.jsonl`: 6 sprints valid.
- `validate-plan --ci`: 4 tracked phase files valid.
- `just lint`: `lint.ci.consumer: pass (1 steps)`.
- `just test`: `test: pass (1 steps)`.
- All six live description/acceptance fields contain no retired GH-136/GH-198/GH-786 trace IDs or draft ANC labels; anchors.json and anchors.md are unchanged against the r1 head.

A new planning judge run is required after the lead resolves scope triage. ANC-018/020/021 still await their separate definition work and were not judged in r1.
