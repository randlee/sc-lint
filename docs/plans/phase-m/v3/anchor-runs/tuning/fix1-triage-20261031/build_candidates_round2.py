import json
# Round 2. ANC-010: R1-R3 broke the approved fixtures (they describe outcomes, not crate work), so R4-R6 keep R0's
# outcome-based framing and add only the context rule; addressed reopened (A1-A3) for sub-0.6 cells on C-CTX-2,
# P-CONFLICT-1 and the m-1 diagnostic. ANC-009: K4-K6 for the sub-0.6 contradicted cell on P-SHAPE-SILENT. Oracles unchanged.
FORBID = ("confidence", "verdict", "role-to-verdict", "classify the three")
def q(instr, yes, no, unclear):
    x = {"type": "choice", "instructions": instr, "criteria": {"yes": yes, "no": no, "unclear": unclear}}
    low = json.dumps(x).lower(); assert not any(p in low for p in FORBID), instr[:40]; return x
def load(a): return json.load(open(f"{a}/candidates.json"))
def save(a, C): json.dump(C, open(f"{a}/candidates.json", "w"), indent=1, sort_keys=True)
C = load("ANC-010"); R = C["candidates"]["relevant"]; A = C["candidates"]["addressed"]; R0 = R["R0"]
CTX = ("An item that mentions a site allowance only as the route other shapes or remaining findings take, as something another item delivers or tests, "
       "or only in requirement or documentation wording, without itself setting how PORT-010 treats an annotated item, is a different subject.")
R["R4"] = q(R0["instructions"].replace(" Judge overlap of subject matter only.", " " + CTX + " Judge overlap of subject matter only."),
  R0["criteria"]["yes"] + " An item that states whether PORT-010 reports an item carrying allow(PORT-010) is yes.",
  R0["criteria"]["no"].replace("concerns allowances for other rules only,", "concerns allowances for other rules only, mentions the PORT-010 allowance only as another item's work or in docs wording,"),
  R0["criteria"]["unclear"])
R["R5"] = q("Does `item.text` itself set, add, change or remove whether or how PORT-010 honours a per-site `#[sc_lint(allow(...))]` attribute, including stating that an annotated item is or is not reported, parsing allow(PORT-010), or replacing per-item PORT-010 allowances with another mechanism? " + CTX + " PORT-010 work that only recognises companion shapes or keeps real gaps reported, and allowances for other rules, are also different subjects. Judge overlap of subject matter only. Do not judge whether the item obeys `anchor.statement`.",
  "The item itself sets or changes PORT-010's handling of a per-site allow(PORT-010) attribute, or states whether an annotated item is reported.",
  "The item does not: it works on other PORT-010 shapes, other rules or other subjects, and any mention of the allowance is context, another item's work, or docs wording.",
  "The item text is too vague to tell whether it sets PORT-010 site-allowance handling.")
R["R6"] = q("`anchor.scope` is per-site allowance of PORT-010 findings through #[sc_lint(allow(...))]. Does `item.text` plan or state anything about how PORT-010 treats an item carrying such an attribute (honouring it, ignoring it, parsing it, removing or replacing it, or whether the annotated item is reported)? " + CTX + " Judge subject presence only, whatever the item plans; do not judge whether the item obeys `anchor.statement`.",
  "The item plans or states PORT-010's treatment of an item carrying a per-site allow(PORT-010) attribute, in any direction.",
  "The item plans or states nothing about that treatment; it is about other shapes, other rules or other work, and any allowance mention is context, another item's work, or docs wording.",
  "The item mentions the allowance in a way that cannot be placed as its own plan or only context.")
A0 = A["A0"]
A["A1"] = q(A0["instructions"] + " Referring to an allowance that another item delivers, or using the allowance as the route for findings this item does not handle, does not commit to the outcome.",
  A0["criteria"]["yes"], A0["criteria"]["no"].replace("uses another suppression mechanism,", "uses another suppression mechanism, only refers to an allowance another item delivers,"), A0["criteria"]["unclear"])
A["A2"] = q("Does `item.text` itself commit that PORT-010 does not report an item carrying `#[sc_lint(allow(PORT-010), reason = \"...\")]`, as new work or by keeping it working? Accept honour, skip, suppress, exempt, records no finding or not reported for the annotated item. It does not commit when it only investigates or documents allowances, only refers to an allowance another item delivers, changes the attribute without stating the outcome, or says annotated items are still reported or the attribute is ignored. Judge presence of the outcome only; do not judge conflict.",
  "The item states that PORT-010 does not report, or keeps not reporting, an item carrying an allow(PORT-010) site attribute with a reason.",
  "The item does not state that outcome as its own commitment.",
  "The item names an outcome for an annotated item, but its wording leaves open whether PORT-010 still reports it.")
A["A3"] = q("Is there a sentence in `item.text` committing, as this item's own outcome, that PORT-010 leaves an item carrying `#[sc_lint(allow(PORT-010), reason = \"...\")]` unreported? Pointers to an allowance delivered elsewhere, investigation, docs wording, attribute changes with no stated outcome, and plans where annotated items stay reported or the attribute is ignored contain no such commitment. Judge only whether the commitment is written; do not judge conflict.",
  "Such a commitment is written in the item.", "No such commitment is written.", "A sentence touches this but cannot be read as the commitment or not.")
C["bundles"].update({"B4": {"relevant": "R4", "addressed": "A1", "contradicted": "K0"}, "B5": {"relevant": "R5", "addressed": "A2", "contradicted": "K0"},
                     "B6": {"relevant": "R6", "addressed": "A3", "contradicted": "K0"}}); save("ANC-010", C)
C = load("ANC-009"); K = C["candidates"]["contradicted"]
SH = ("An item whose #[cfg(unix)] item has a recognised counterpart, such as a same-name #[cfg(not(unix))] sibling, a differently named Windows twin or code compiled only on Unix, "
      "has a companion or fallback; recognising such a shape leaves real gaps reported and is not conflict. ")
K["K4"] = q(SH + K["K1"]["instructions"], K["K1"]["criteria"]["yes"], K["K1"]["criteria"]["no"], K["K1"]["criteria"]["unclear"])
K["K5"] = q(SH + K["K2"]["instructions"], K["K2"]["criteria"]["yes"], K["K2"]["criteria"]["no"], K["K2"]["criteria"]["unclear"])
K["K6"] = q("Does `item.text` plan a blanket suppression of PORT-010 findings: skipping #[cfg(unix)] items by file, module, path, crate, name pattern or a switch, regardless of whether each has a companion, a portable fallback or its own per-site allowance? Not blanket suppression: recognising a specific companion shape (for example a same-name #[cfg(not(unix))] sibling), honouring a reasoned allowance on one annotated item, silence, or unrelated work. Judge conflict only.",
  "The item plans such a blanket suppression.", "The item plans no blanket suppression.", "The item plans a suppression whose reach cannot be told.")
C["bundles"].update({"B4": {"relevant": "R0", "addressed": "A2", "contradicted": "K4"}, "B5": {"relevant": "R0", "addressed": "A2", "contradicted": "K5"},
                     "B6": {"relevant": "R0", "addressed": "A2", "contradicted": "K6"}}); save("ANC-009", C)
print("ok")
