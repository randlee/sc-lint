import json
# Round-1 candidates for the three question_defect tournaments. B0 = current questions (ANC-010 override 3b9e9da4...;
# ANC-012 and ANC-009 default plan.json v1 questions). Candidates derive from the anchor statement/scope only.
FORBID = ("confidence", "verdict", "role-to-verdict", "classify the three")
def q(instr, yes, no, unclear):
    x = {"type": "choice", "instructions": instr, "criteria": {"yes": yes, "no": no, "unclear": unclear}}
    low = json.dumps(x).lower(); assert not any(p in low for p in FORBID), instr[:40]; return x
def load(a): return json.load(open(f"{a}/candidates.json"))
def save(a, C): json.dump(C, open(f"{a}/candidates.json", "w"), indent=1, sort_keys=True)

# ---------- ANC-010: reopen relevant only
C = load("ANC-010"); R = C["candidates"]["relevant"]
CTX10 = ("Mentioning a site allowance only as the route that other shapes or remaining findings take, as work another item delivers, "
         "or describing it in requirement or documentation text, without changing how code in the listed crates parses or honours "
         "#[sc_lint(allow(...))] for PORT-010, is not this subject.")
R["R1"] = q("Does the work in `item.text` add, change or remove how a per-site `#[sc_lint(allow(...))]` attribute allows PORT-010 findings, or set whether PORT-010 honours such an attribute, in crates/sc-lint-portability, crates/sc-lint-directives or crates/sc-lint-attributes? " + CTX10 + " Judge overlap of subject matter only. Do not judge whether the item obeys `anchor.statement`.",
  "The item itself adds, changes, removes or reworks PORT-010's handling of a per-site #[sc_lint(allow(...))] allowance, including parsing allow(PORT-010) or replacing per-item PORT-010 allowances with another mechanism.",
  "The item does not do that work: it changes which shapes PORT-010 reports, mentions the allowance only as another item's work or as where other cases go, updates wording about it in docs or requirements only, concerns allowances for other rules, or is other work.",
  "The item text is too vague to tell whether it changes PORT-010 site-allowance handling itself.")
R["R2"] = q("List what `item.text` itself builds or changes. Is one of those things PORT-010's handling of a per-site `#[sc_lint(allow(...))]` attribute: parsing allow(PORT-010), whether PORT-010 honours it, or replacing it with another mechanism? " + CTX10 + " Judge subject presence only.",
  "One thing the item builds or changes is PORT-010's handling of a per-site #[sc_lint(allow(...))] allowance.",
  "Nothing the item builds or changes is PORT-010 site-allowance handling; any allowance mention is context, another item's work, or docs or requirement wording.",
  "The item mentions PORT-010 site allowances in a way that cannot be placed as its own work or only context.")
R["R3"] = q("`anchor.scope` covers work that adds or changes per-site allowance of PORT-010 findings through #[sc_lint(allow(...))] in crates/sc-lint-portability, crates/sc-lint-directives or crates/sc-lint-attributes. Is that work part of `item.text`, in either direction (honouring, ignoring, removing or replacing the allowance)? " + CTX10 + " Judge subject presence only, whatever the item plans.",
  "The item plans code work on PORT-010 site-allowance handling in those crates, in any direction.",
  "The item plans no such work: it works on other PORT-010 shapes, other rules or other subjects, and any allowance mention is context, another item's work, or docs wording.",
  "It cannot be told whether the item plans code work on PORT-010 site-allowance handling.")
C["bundles"].update({f"B{i}": {"relevant": f"R{i}", "addressed": "A0", "contradicted": "K0"} for i in (1, 2, 3)}); save("ANC-010", C)

# ---------- ANC-012: all three roles
C = load("ANC-012"); c = C["candidates"]
SUBJ12 = ("`anchor.scope` covers consumer configuration keys in sc-lint.toml for setting up or running steps: setup or hook keys, interpreter-selection keys, "
          "and per-step execution settings such as a lint step's env or cwd.")
c["relevant"]["R1"] = q(SUBJ12 + " Does `item.text` add, change or remove such a key, or decide which of them exist? Judge subject overlap only; do not judge whether the item obeys `anchor.statement`.",
  "The item adds, changes, removes or rules on a consumer setup, hook, interpreter-selection or per-step execution key in sc-lint.toml (for example env or cwd on a lint step, a setup or runner table).",
  "The item adds or changes no consumer configuration key of that kind: it changes analyzer rules, output or logging, or other work.",
  "The item text is too vague to tell whether it touches such configuration keys.")
c["relevant"]["R2"] = q("Does the work in `item.text` touch how consumers configure the setup or execution of sc-lint steps through keys in sc-lint.toml, such as a lint step's env or cwd, a setup or hook table, or an interpreter or runner key? Judge subject overlap only, not whether the item obeys `anchor.statement`.",
  "The item adds, changes, removes or keeps out such a key.",
  "The item touches no such key: it is about analyzer rules, report or log output, or other work.",
  "The item text is too vague to tell.")
c["relevant"]["R3"] = q("`anchor.statement` forbids adding a setup-hook or interpreter-selection key to sc-lint.toml. Work that adds or changes how consumers configure step execution there (env, cwd, setup, hooks, runner or interpreter keys) is on this subject, whether or not it follows the statement. Is `item.text` on this subject?",
  "Yes: the item adds, changes or rules on step-execution, setup, hook or interpreter configuration keys in sc-lint.toml.",
  "No: the item changes no such configuration; it is about analyzer rules, output, logging or other work.",
  "The item text is too vague to tell.")
NO12 = "The item does not say that, or it adds such a key."
c["addressed"]["A1"] = q("Does `item.text` state that no setup-hook or interpreter-selection configuration key is added to sc-lint.toml, for example that [tool.sc-lint].setup and [tool.sc-lint].runner stay rejected? Judge whether this commitment is stated; an item that adds such a key does not state it.",
  "The item states that no setup or interpreter-selection key is added, or that setup and runner keys stay rejected or unknown.",
  "The item does not state that: it is silent about such keys, or it adds a setup, hook or interpreter key.",
  "The item mentions such keys but it cannot be told whether it keeps them out.")
c["addressed"]["A2"] = q("`anchor.statement` forbids a setup-hook or interpreter-selection configuration key. Does `item.text` explicitly keep that prohibition, by saying such keys are not added or stay rejected? An item that adds such a key does not keep it. Judge presence of the explicit commitment only.",
  "The item explicitly says no setup-hook or interpreter-selection key is added, or that keys such as setup or runner stay rejected.",
  "The item says no such thing, or it adds a setup, hook or interpreter key.",
  "The item touches such keys in wording that cannot be read either way.")
c["addressed"]["A3"] = q("Is there a sentence in `item.text` committing that sc-lint.toml gains no setup-hook key and no interpreter-selection key (for example setup or runner stay unknown keys)? Plans that add such a key contain no such commitment. Judge only whether the commitment is written.",
  "Such a commitment is written in the item.",
  "No such commitment is written, or the item adds a setup, hook or interpreter key.",
  "A sentence touches this but cannot be read as the commitment or not.")
CONF12 = ("Conflict means the item adds or keeps a sc-lint.toml key that runs setup or hook commands (for example [tool.sc-lint].setup.pre or a per-step before or pre hook) "
          "or that selects the interpreter or runner sc-lint uses for a step (for example [tool.sc-lint].runner.python or a per-step python key). "
          "Not conflict: env or cwd on a step, env values passed verbatim to the child even if they name an interpreter, rejecting setup or runner keys, silence, or unrelated work.")
c["contradicted"]["K1"] = q("Does `item.text` plan a setup-hook or interpreter-selection configuration key that `anchor.statement` forbids? " + CONF12 + " Judge conflict only.",
  "The item adds or keeps such a setup-hook or interpreter-selection key.", "The item adds no such key.", "The item describes a key whose role as a setup hook or interpreter selection cannot be told.")
c["contradicted"]["K2"] = q("Would `item.text`, as planned, add a key to sc-lint.toml through which sc-lint runs setup or hook commands, or picks the interpreter or runner for a step? " + CONF12,
  "Yes, such a key is added or kept.", "No such key is added.", "It cannot be told whether a planned key runs setup or hook commands or selects an interpreter.")
c["contradicted"]["K3"] = q(CONF12 + " Does `item.text` plan such a conflicting key? Judge conflict only.",
  "The item plans a setup-hook or interpreter-selection key.", "The item plans no such key.", "The item plans a key whose role cannot be told.")
C["bundles"].update({f"B{i}": {"relevant": f"R{i}", "addressed": f"A{i}", "contradicted": f"K{i}"} for i in (1, 2, 3)}); save("ANC-012", C)

# ---------- ANC-009: addressed and contradicted (relevant default held)
C = load("ANC-009"); c = C["candidates"]
ALLOW = ("A reasoned per-site allowance that covers only the one annotated item (for example #[sc_lint(allow(PORT-010), reason = \"...\")]) and recognising a specific companion shape "
         "are not blanket suppression.")
c["addressed"]["A1"] = q("Does `item.text` state that PORT-010 keeps reporting real parity gaps, that is a production #[cfg(unix)] item with no #[cfg(windows)] companion and no portable fallback (and, where an allowance exists, no allowance of its own)? Judge whether that commitment is stated; an item that suppresses such items broadly does not state it.",
  "The item states that such items, or unannotated or unpaired ones, are still reported, for example through negative tests.",
  "The item does not state that: it is silent about real gaps, or it suppresses them broadly.",
  "The item touches real gaps in wording that cannot be read as the commitment or not.")
c["addressed"]["A2"] = q("`anchor.statement` requires that a production #[cfg(unix)] item with no #[cfg(windows)] companion and no portable fallback still be reported. Does `item.text` explicitly keep that, for example by saying unpaired or unannotated items or negative cases stay reported? A plan that suppresses many items at once does not keep it. Judge presence only.",
  "The item explicitly says real gaps (unpaired, unannotated or negative-case items) are still reported.",
  "The item does not say so, or it suppresses such items broadly.",
  "The item mentions this in wording that cannot be read either way.")
c["addressed"]["A3"] = q("Is there a sentence in `item.text` committing that PORT-010 still reports a #[cfg(unix)] item that has no companion, no portable fallback and no allowance of its own? Plans that skip such items by file, path, name or switch contain no such commitment. Judge only whether the commitment is written.",
  "Such a commitment is written, for example as a still-reported negative or an unannotated sibling that is still reported.",
  "No such commitment is written, or the item skips such items broadly.",
  "A sentence touches this but cannot be read as the commitment or not.")
c["contradicted"]["K1"] = q("Does `item.text` plan to blanket-suppress real parity gaps, so that PORT-010 stops reporting #[cfg(unix)] items that have no companion, no portable fallback and no allowance of their own? Examples: skipping every item in a file, module, path or crate, a repository-wide switch or exclusion, or name heuristics. " + ALLOW + " Silence and unrelated work are not conflict. Judge conflict only.",
  "The item plans such a blanket suppression.", "The item plans no blanket suppression.", "The item plans a suppression whose reach cannot be told.")
c["contradicted"]["K2"] = q(ALLOW + " Does `item.text` instead plan something that leaves real parity gaps unreported in bulk, such as a file-, module-, path- or crate-wide allowance or exclusion, a switch, or a name heuristic, so that #[cfg(unix)] items with no companion, no portable fallback and no allowance of their own are not reported? Judge conflict only.",
  "Yes, real gaps are left unreported in bulk.", "No: the item suppresses nothing in bulk, or only per annotated item or per recognised shape, or is silent.", "The reach of a planned suppression cannot be told.")
c["contradicted"]["K3"] = q("Would some production #[cfg(unix)] item that has no #[cfg(windows)] companion, no portable fallback and no per-site allowance of its own stop being reported under the plan in `item.text`? " + ALLOW + " Judge conflict only.",
  "Yes: the plan stops reporting such an unannotated, unpaired item, for example by file, path, name or switch.", "No: only annotated items or recognised companion shapes stop being reported, or the item is silent or unrelated.", "It cannot be told whether such an item would stop being reported.")
C["bundles"].update({f"B{i}": {"relevant": "R0", "addressed": f"A{i}", "contradicted": f"K{i}"} for i in (1, 2, 3)}); save("ANC-009", C)
print("ok")
