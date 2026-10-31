import json, hashlib, sys
sys.path.insert(0, "/Users/randlee/github/sc-lint-worktrees/plan/phase-m/.claude/skills/plan-anchoring/scripts")
import anchor_tuning_contract as T
# Oracles for the fix1 plan-gate triage, frozen before any Jev call of this triage. Expectations derive only from the
# locked anchors in anchors.json (70449b71...) and intent.md (be7bdec8..., phase-m-intent-v3 as cut 2026-10-31).
V = "/Users/randlee/github/sc-lint-worktrees/plan/phase-m/docs/plans/phase-m/v3"
A = {r["id"]: r for r in json.load(open(f"{V}/anchors.json"))["anchors"]}
INTENT_SHA = hashlib.sha256(open(f"{V}/intent.md", "rb").read()).hexdigest()
assert INTENT_SHA == "be7bdec8ba9ae053efdbef2386bf6334f67dadd97a9800e0391a125322ad1241"
def case(cid, kind, title, text, roles, verdict, why):
    r, a, c = roles
    return {"case_id": cid, "kind": kind, "item": {"title": title, "text": text},
            "expected": {"relevant": r, "addressed": a, "contradicted": c}, "expected_verdict": verdict, "rationale": why}
def write(path, o):
    b = T.canonical_bytes(o); open(path, "wb").write(b); h = hashlib.sha256(b).hexdigest()
    open(path.replace(".json", ".sha256"), "w").write(f"{h} {len(o['cases'])}\n"); print(path, h, len(o["cases"]))
def header(aid):
    a = A[aid]; assert a["status"] == "locked"
    return {"schema": "plan-anchor/tuning-oracle/2", "gate": "planning", "anchor_id": aid, "anchor_version": a["version"], "kind": a["kind"],
            "anchor_record_sha256": T.sha256(a), "intent_id": "phase-m-intent-v3", "intent_sha256": INTENT_SHA, "repeat_count": 2}

# ---- ANC-010 (directive): append-only successor of planning-v3 oracle 046514db...
V1 = f"{V}/anchor-runs/tuning/ANC-010/planning-v3/oracle.json"; raw = open(V1, "rb").read()
assert hashlib.sha256(raw).hexdigest() == "046514db9ed23f273ee5898ca24d14a99ed60c0e4d954727923f9126ce6eb1bd"
o1 = json.loads(raw); open("ANC-010/oracle-v1.json", "wb").write(raw)
NA = ("no", "no", "no"); SAT = ("yes", "yes", "no")
SC10 = "ANC-010 v3 scope: 'The item adds or changes per-site allowance of PORT-010 findings through #[sc_lint(allow(...))], in crates/sc-lint-portability, crates/sc-lint-directives or crates/sc-lint-attributes.' "
new10 = [
 case("C-CTX-1", "out_of_scope_refers_to_other_sprint_allowance", "PORT-010 same-name siblings (#786 S3)",
  "In crates/sc-lint-portability, PORT-010 does not report a #[cfg(unix)] item with a same-name #[cfg(not(unix))] sibling in the same module. Other #786 shapes are not changed here; they use the per-site PORT-010 allowance that a later sprint delivers.",
  NA, "not_applicable", SC10 + "Naming another sprint's allowance as the route for other shapes neither adds nor changes the allowance."),
 case("C-CTX-2", "out_of_scope_conditional_refers_to_allowance", "PORT-010 Unix-only modules (#786 S6)",
  "In crates/sc-lint-portability, PORT-010 does not report the declaration #[cfg(unix)] mod x; of a module never compiled on Windows, or the non-public items in its files. Whether pub items are covered waits on a ruling; if they are not, a pub item stays a finding, and its reasoned site allowance is tested in the allowance sprint.",
  NA, "not_applicable", SC10 + "Pointing a remaining finding at the allowance another sprint provides does not change allowance handling."),
 case("C-CTX-3", "out_of_scope_requirements_wording", "Reconcile PORT-010 requirement bullets",
  "Update the PORT-010 bullets in docs/requirements.md to say that S3 and S6 are recognised and that other shapes are handled through a per-site allowance. No code in crates/ changes.",
  NA, "not_applicable", SC10 + "A requirements-text reconciliation with no change to allowance handling in the listed crates is outside scope."),
 case("C-ADD-1", "compliant_shapes_plus_allowance", "PORT-010 S3 plus site allowance",
  "In crates/sc-lint-portability, PORT-010 does not report a #[cfg(unix)] item with a same-name #[cfg(not(unix))] sibling in the same module. PORT-010 also honours #[sc_lint(allow(PORT-010), reason = \"...\")] on a single item: the annotated item is not reported, and an unannotated sibling still is.",
  SAT, "satisfied", "ANC-010 v3 statement: 'an item carrying #[sc_lint(allow(PORT-010), reason = \"...\")] is not reported by PORT-010'; adding that handling is in scope and states the outcome."),
]
o10 = dict(o1); o10["oracle_version"] = 2
o10["supersedes"] = {"oracle_sha256": "046514db9ed23f273ee5898ca24d14a99ed60c0e4d954727923f9126ce6eb1bd", "relation": "append_only", "v1_case_ids": [c["id"] for c in o1["cases"]]}
o10["intent_sha256_successor"] = INTENT_SHA; o10["anchor_record_sha256_current"] = T.sha256(A["ANC-010"])
o10["cases"] = o1["cases"] + new10
for x, y in zip(o1["cases"], o10["cases"]): assert T.canonical_bytes(x) == T.canonical_bytes(y)
write("ANC-010/oracle.json", o10)

# ---- ANC-012 (prohibition): new oracle; no prior oracle or override exists.
SC12 = "ANC-012 v2 scope: 'The item adds or changes consumer setup, hook or interpreter-selection configuration keys in sc-lint.toml.' "
FX12 = A["ANC-012"]["fixtures"]
c12 = [
 case("FX-PASS-1", "fixture_pass", "ANC-012 pass fixture", FX12["pass"][0], ("yes", "yes", "no"), "satisfied", "ANC-012 v2 approved fixtures.pass."),
 case("FX-FAIL-1", "fixture_fail", "ANC-012 fail fixture", FX12["fail"][0], ("yes", "no", "yes"), "contradicted", "ANC-012 v2 approved fixtures.fail: adds an interpreter-selection key."),
 case("P-COMPLY-1", "compliant_explicit", "Lint-step env and cwd (#198)",
  "A [[tool.sc-lint.lint]] step in sc-lint.toml accepts an env table and a cwd key, applied to that step's child process. [tool.sc-lint].setup and [tool.sc-lint].runner stay rejected as unknown keys; no setup-hook or interpreter-selection key is added.",
  ("yes", "yes", "no"), "satisfied", SC12 + "Same shape as the pass fixture; INTENT #198: 'Lint steps accept env and cwd ... No setup hook or interpreter-selection key is added'."),
 case("P-SILENT-1", "silent", "Lint-step env and cwd",
  "A [[tool.sc-lint.lint]] step in sc-lint.toml accepts an env table and a cwd key, applied to that step's child process, with the same meaning they have on a test command selector.",
  ("yes", "no", "no"), "satisfied", SC12 + "Changes consumer step configuration keys (pass-fixture subject) without stating the prohibition; silence is not conflict."),
 case("P-BOUNDARY-1", "boundary_env_value_verbatim", "Lint-step env passes PYO3_PYTHON verbatim",
  "A [[tool.sc-lint.lint]] step accepts env = { PYO3_PYTHON = \".venv/bin/python\" }; sc-lint passes the value to the child verbatim and does not resolve or select an interpreter. No setup or runner key is added; both stay rejected.",
  ("yes", "yes", "no"), "satisfied", SC12 + "An env value is consumer data handed to the child, not a sc-lint interpreter-selection key; INTENT #198 and ADR-022 decision 6."),
 case("P-CONFLICT-1", "conflict_setup_hook", "Consumer setup hook",
  "Add [tool.sc-lint].setup.pre, a list of commands that sc-lint runs before the first lint step, so consumers can create their Python venv.",
  ("yes", "no", "yes"), "contradicted", "ANC-012 v2 statement names [tool.sc-lint].setup.pre as a forbidden setup-hook key."),
 case("P-CONFLICT-2", "conflict_step_interpreter", "Per-step interpreter key",
  "A [[tool.sc-lint.lint]] step gains a python key; sc-lint runs the step's argv with that interpreter instead of the one on PATH.",
  ("yes", "no", "yes"), "contradicted", "ANC-012 v2 forbids an interpreter-selection configuration key; INTENT out of scope: 'interpreter selection'."),
 case("P-CONFLICT-3", "conflict_step_hook", "Per-step pre-run hook",
  "A [[tool.sc-lint.lint]] step gains a before key holding a command that sc-lint runs before that step's child process starts.",
  ("yes", "no", "yes"), "contradicted", "ANC-012 v2 forbids a setup-hook configuration key; INTENT out of scope: 'setup hooks'."),
 case("P-IRR-1", "irrelevant_boundary_rule", "SCB-MANIFEST-004 local paths (#136)",
  "SCB-MANIFEST-004 does not report a self::, super:: or crate:: path, or a local mod, named like an owned dependency.",
  NA, "not_applicable", SC12 + "Boundary rule change; no sc-lint.toml consumer keys."),
 case("P-IRR-2", "irrelevant_step_logs", "Failed-step output and step logs (#195)",
  "Failed lint steps print both stdout and stderr, and every executed step writes both streams to <log-root>/steps/<step>.log under the existing --log-root.",
  NA, "not_applicable", SC12 + "Output and logging behaviour; no consumer configuration key is added or changed."),
 case("P-IRR-3", "irrelevant_port010", "PORT-010 same-name siblings",
  "In crates/sc-lint-portability, PORT-010 does not report a #[cfg(unix)] item with a same-name #[cfg(not(unix))] sibling in the same module.",
  NA, "not_applicable", SC12 + "Portability rule; outside scope."),
]
o12 = header("ANC-012"); o12["oracle_version"] = 1; o12["cases"] = c12
o12["excluded_probes"] = [{"kind": "topical_only_docs", "reason": "An item that only documents that no setup or runner key exists changes no key; ANC-012 v2 does not fix whether that is in scope."}]
write("ANC-012/oracle.json", o12)

# ---- ANC-009 (prohibition): new oracle; no prior oracle or override exists.
SC9 = "ANC-009 v2 scope: 'The item changes when PORT-010 reports or stops reporting a #[cfg(unix)] item.' "
AL = "INTENT (cut 2026-10-31): 'Other shapes use reasoned site allowances; unallowed real parity gaps still report' and ANC-010 v3 (locked): an item carrying #[sc_lint(allow(PORT-010), reason)] is not reported. A reasoned single-item allowance is therefore not blanket suppression. "
FX9 = A["ANC-009"]["fixtures"]
c9 = [
 case("FX-PASS-1", "fixture_pass", "ANC-009 pass fixture", FX9["pass"][0], ("yes", "yes", "no"), "satisfied", "ANC-009 v2 approved fixtures.pass."),
 case("FX-FAIL-1", "fixture_fail", "ANC-009 fail fixture", FX9["fail"][0], ("yes", "no", "yes"), "contradicted", "ANC-009 v2 approved fixtures.fail."),
 case("P-ALLOW-EXPLICIT", "compliant_site_allow", "PORT-010 per-site allowance",
  "In crates/sc-lint-portability, PORT-010 does not report an item carrying #[sc_lint(allow(PORT-010), reason = \"...\")]; the reason is required, and the allowance covers only that item, not its module, file or siblings. An unannotated production #[cfg(unix)] item with no #[cfg(windows)] companion and no portable fallback is still reported.",
  ("yes", "yes", "no"), "satisfied", SC9 + AL),
 case("P-ALLOW-SILENT", "silent_site_allow", "PORT-010 per-site allowance",
  "In crates/sc-lint-portability, PORT-010 does not report an item carrying #[sc_lint(allow(PORT-010), reason = \"...\")].",
  ("yes", "no", "no"), "satisfied", SC9 + AL + "The item does not restate that real gaps stay reported; silence is not conflict."),
 case("P-SHAPE-EXPLICIT", "compliant_shape", "PORT-010 same-name siblings",
  "In crates/sc-lint-portability, PORT-010 does not report a #[cfg(unix)] item with a same-name #[cfg(not(unix))] sibling in the same module. A #[cfg(unix)] item with no companion and no portable fallback is still reported (negative tests).",
  ("yes", "yes", "no"), "satisfied", SC9 + "Narrow companion recognition that keeps real gaps reported."),
 case("P-SHAPE-SILENT", "silent_shape", "PORT-010 same-name siblings",
  "In crates/sc-lint-portability, PORT-010 does not report a #[cfg(unix)] item with a same-name #[cfg(not(unix))] sibling in the same module.",
  ("yes", "no", "no"), "satisfied", SC9 + "Narrow companion recognition; silent about real gaps; not a blanket suppression."),
 case("P-CONFLICT-FILE", "conflict_file_allow", "PORT-010 file-level allowance",
  "One #![sc_lint(allow(PORT-010))] at the top of a file stops PORT-010 reporting every #[cfg(unix)] item in that file.",
  ("yes", "no", "yes"), "contradicted", SC9 + "Suppresses real gaps for a whole file; INTENT out of scope: 'Blanket suppression'."),
 case("P-CONFLICT-PATH", "conflict_path_exclusion", "PORT-010 path exclusion",
  "sc-lint.toml gains a PORT-010 exclude list; PORT-010 skips every #[cfg(unix)] item under a listed path such as src/platform/**.",
  ("yes", "no", "yes"), "contradicted", SC9 + "INTENT out of scope: 'a repository-wide exclusion'."),
 case("P-CONFLICT-NAME", "conflict_name_heuristic", "PORT-010 skips unix-named functions",
  "PORT-010 stops reporting any #[cfg(unix)] function whose name ends in _unix, whether or not it has a Windows companion or portable fallback.",
  ("yes", "no", "yes"), "contradicted", "ANC-009 v2 statement: a #[cfg(unix)] item with no companion and no fallback must still be reported."),
 case("P-IRR-1", "irrelevant_config", "Lint-step env and cwd",
  "A [[tool.sc-lint.lint]] step in sc-lint.toml accepts an env table and a cwd key, applied to that step's child process.",
  NA, "not_applicable", SC9 + "No PORT-010 change."),
 case("P-IRR-2", "irrelevant_other_rule", "SCB-CYCLE-001 path filtering",
  "SCB-CYCLE-001 accepts a path filter so cycles inside excluded test directories are not reported.",
  NA, "not_applicable", SC9 + "Another rule's filtering; PORT-010 unchanged."),
]
o9 = header("ANC-009"); o9["oracle_version"] = 1; o9["cases"] = c9
write("ANC-009/oracle.json", o9)
