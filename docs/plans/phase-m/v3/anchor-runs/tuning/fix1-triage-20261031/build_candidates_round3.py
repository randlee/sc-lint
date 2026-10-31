import json
# Round 3, ANC-010 only. R4 is exact (min 0.79). Addressed still has sub-0.6 or missed cells on P-CONFLICT-1 (PORT-010
# ignores the attribute; exclude glob instead), and held K0 dips to 0.55 on P-TOPICAL (deferred behaviour).
# A4-A6 state that ignoring the attribute leaves the item reported; K1-K3 state that deferring the behaviour is not conflict.
FORBID = ("confidence", "verdict", "role-to-verdict", "classify the three")
def q(instr, yes, no, unclear):
    x = {"type": "choice", "instructions": instr, "criteria": {"yes": yes, "no": no, "unclear": unclear}}
    low = json.dumps(x).lower(); assert not any(p in low for p in FORBID), instr[:40]; return x
C = json.load(open("ANC-010/candidates.json")); A = C["candidates"]["addressed"]; K = C["candidates"]["contradicted"]
IGN = (" If the item makes PORT-010 ignore or drop the attribute, or uses an exclusion or other mechanism instead of it, the annotated item is still reported by PORT-010, so the outcome is not committed.")
A["A4"] = q(A["A2"]["instructions"] + IGN, A["A2"]["criteria"]["yes"], A["A2"]["criteria"]["no"], A["A2"]["criteria"]["unclear"])
A["A5"] = q(A["A0"]["instructions"] + IGN + " Referring to an allowance another item delivers does not commit either.",
  A["A0"]["criteria"]["yes"], A["A0"]["criteria"]["no"].replace("uses another suppression mechanism,", "makes PORT-010 ignore the attribute, uses an exclusion or another mechanism instead, only refers to an allowance another item delivers,"), A["A0"]["criteria"]["unclear"])
A["A6"] = q("After the work in `item.text` is done, as this item states it, does PORT-010 leave an item carrying `#[sc_lint(allow(PORT-010), reason = \"...\")]` unreported? Answer yes only when the item itself states that outcome (honour, skip, suppress, exempt, records no finding, not reported). Answer no when the item does not state it, only investigates, documents or refers to an allowance another item delivers, or plans that PORT-010 ignores or drops the attribute or uses an exclusion instead, since then the annotated item is still reported. Judge the stated outcome only; do not judge conflict.",
  "The item states that PORT-010 leaves the annotated item unreported.", "The item does not state that outcome, or its plan leaves the annotated item reported.", "The item names an outcome for an annotated item whose wording leaves it open.")
DEF = " Leaving the allowance behaviour to be decided later, reworking it without stating an outcome, or deferring it is not a conflict."
K["K1"] = q(K["K0"]["instructions"].replace(" Judge conflict only.", DEF + " Judge conflict only."), K["K0"]["criteria"]["yes"],
  K["K0"]["criteria"]["no"].replace("whether or not the item mentions site allowances.", "whether or not the item mentions site allowances, including items that defer or leave the allowance behaviour to be decided."), K["K0"]["criteria"]["unclear"])
K["K2"] = q("Does `item.text` state that PORT-010 still reports an item carrying `#[sc_lint(allow(PORT-010), reason = \"...\")]`, that PORT-010 ignores, drops or refuses that site allowance, or that per-item PORT-010 allowances will not be provided and a broader mechanism is used instead?" + DEF + " Silence, investigation, requiring the reason, or limiting the allowance to the annotated item or to PORT-010 is not a conflict either. Judge conflict only.",
  K["K0"]["criteria"]["yes"], "The item states none of those: it honours the allowance, is silent, investigates, or defers the behaviour.", K["K0"]["criteria"]["unclear"])
K["K3"] = q("Conflict with `anchor.statement` means the item states a plan in which an item carrying `#[sc_lint(allow(PORT-010), reason = \"...\")]` stays reported: PORT-010 ignores, drops or refuses the attribute, or per-item allowances are replaced by a broader mechanism." + DEF + " Silence, investigation, requiring the reason and per-item or per-rule limits are not conflict. Does `item.text` state such a conflicting plan? Judge conflict only.",
  "The item states such a conflicting plan.", "The item states no such plan.", "The item says something about PORT-010 allowances that might or might not leave an annotated item reported.")
C["bundles"].update({"B7": {"relevant": "R4", "addressed": "A4", "contradicted": "K0"}, "B8": {"relevant": "R4", "addressed": "A5", "contradicted": "K0"},
                     "B9": {"relevant": "R4", "addressed": "A6", "contradicted": "K0"}, "B10": {"relevant": "R4", "addressed": "A2", "contradicted": "K1"},
                     "B11": {"relevant": "R4", "addressed": "A2", "contradicted": "K2"}, "B12": {"relevant": "R4", "addressed": "A2", "contradicted": "K3"}})
json.dump(C, open("ANC-010/candidates.json", "w"), indent=1, sort_keys=True); print("ok")
