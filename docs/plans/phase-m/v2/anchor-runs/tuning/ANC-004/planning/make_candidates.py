import json
req=json.load(open('request.json'))
base=req['current_questions']
Q=lambda i,y,n,u:{"type":"choice","instructions":i,"criteria":{"yes":y,"no":n,"unclear":u}}
rel={
"R0":base["relevant"],
"R1":Q("The state holds one planning item (`item.title`, `item.text`) and one anchor about sc-lint's PORT-010 rule, with the area it governs in `anchor.scope`. Does the work in `item.text` change which `#[cfg(unix)]` items PORT-010 reports, or depend directly on such a change? Judge subject overlap only; do not judge whether the item obeys `anchor.statement`.",
 "The item plans, changes, or directly depends on when PORT-010 reports or stops reporting a `#[cfg(unix)]` item, whatever sibling shape it handles.",
 "The item changes a different rule, a different feature, or something other than when PORT-010 reports `#[cfg(unix)]` items, even if it is in the same crate.",
 "The item mentions PORT-010 or portability linting but it is not possible to tell whether it changes which `#[cfg(unix)]` items PORT-010 reports."),
"R2":Q("Does `item.text` describe work that changes when sc-lint's PORT-010 rule reports or stops reporting a `#[cfg(unix)]` item in crates/sc-lint-portability, as described in `anchor.scope`? Work on PORT-010 counts whichever `#[cfg(unix)]` shape or sibling it handles, and an item that decides to leave PORT-010's reporting of such items unchanged also counts. Work on another rule (for example another PORT-0xx rule), on another feature such as dependency ownership or boundaries, or on another crate does not count, even when it is in the same crate. Judge subject overlap only; do not judge whether the item obeys `anchor.statement`.",
 "The item changes, keeps, or directly depends on when PORT-010 reports or stops reporting a `#[cfg(unix)]` item.",
 "The item is about another rule, another feature, or another crate, and does not touch when PORT-010 reports `#[cfg(unix)]` items.",
 "The item mentions portability linting but it is not possible to tell whether PORT-010's reporting of `#[cfg(unix)]` items is affected."),
"R3":Q("Is `item.text` about PORT-010's decision to report or not report `#[cfg(unix)]` items? Signals that it is: the item names PORT-010 and says which `#[cfg(unix)]` functions, match arms or other items it reports, stops reporting, treats as covered, or keeps reporting. Signals that it is not: the item names only a different rule (such as PORT-011), a different feature (such as dependency ownership), or no lint rule at all. Judge subject overlap only, not compliance with `anchor.statement`.",
 "The item names PORT-010 and changes or settles which `#[cfg(unix)]` items it reports.",
 "The item concerns a different rule, a different feature, or no lint rule, and leaves PORT-010's reporting untouched.",
 "The item mentions portability work but does not make clear whether PORT-010's reporting of `#[cfg(unix)]` items is involved."),
}
add={
"A0":base["addressed"],
"A1":Q("Does `item.text` plan for PORT-010 to stop reporting a `#[cfg(unix)]` transport or adapter item when a loopback TCP sibling with no `#[cfg]` guard, used by Windows, sits beside it? A generic mention of PORT-010 false positives, or a different sibling shape, does not count. Judge presence of this plan only, not conflict.",
 "The item says PORT-010 stops reporting, skips, or treats as covered a `#[cfg(unix)]` transport or adapter function or match arm because an unguarded loopback TCP enum variant or function beside it serves Windows.",
 "The item does not say this: it is silent about an unguarded loopback TCP sibling, handles a different sibling shape, speaks only generally, or says PORT-010 keeps reporting the item.",
 "The item mentions a loopback TCP sibling and PORT-010 but it is not possible to tell whether PORT-010 is to stop reporting the `#[cfg(unix)]` item."),
"A2":Q("`anchor.statement` names one shape that PORT-010 must recognize and not report: a `#[cfg(unix)]` transport or adapter function or match arm that sits beside a loopback TCP sibling (an enum variant or a function) that has no `#[cfg]` guard and is what Windows uses. Does `item.text` plan or report that PORT-010 stops reporting this shape? The loopback TCP sibling is the defining feature; other siblings (a same-name `#[cfg(windows)]` function, a `#[cfg(windows)]` named-pipe variant) are different shapes. A general promise to review or fix PORT-010 false positives does not name this shape. Judge presence only; do not judge conflict.",
 "The item says PORT-010 stops reporting, skips, or treats as covered a `#[cfg(unix)]` transport or adapter function or match arm because a loopback TCP enum variant or function without a `#[cfg]` guard beside it serves Windows. Words such as TCP loopback, localhost TCP, TcpLoopback, un-gated or unguarded count. Extra conditions that keep reporting other cases are allowed.",
 "The item does not plan this shape: it is silent about an unguarded loopback TCP sibling, handles only a different sibling shape, speaks only of false positives in general, or says PORT-010 keeps reporting the shape.",
 "The item mentions an unguarded loopback TCP sibling and PORT-010 but it is not possible to tell whether PORT-010 is to stop reporting the `#[cfg(unix)]` item."),
"A3":Q("Look for one specific change in `item.text`: PORT-010 no longer reports a `#[cfg(unix)]` transport or adapter function or match arm when, beside it, there is a loopback TCP enum variant or function that has no `#[cfg]` guard and is what Windows uses. Is that change present in `item.text`? Both parts must be present: (1) PORT-010 stops reporting the `#[cfg(unix)]` item, and (2) the reason is an unguarded loopback TCP sibling. Judge presence only; do not judge conflict.",
 "Both parts are present: PORT-010 stops reporting (or skips, or treats as covered) the `#[cfg(unix)]` transport or adapter item, and the reason given is a loopback TCP sibling with no `#[cfg]` guard that serves Windows.",
 "At least one part is missing: the item does not mention an unguarded loopback TCP sibling, gives a different sibling such as a `#[cfg(windows)]` function or named pipe, only promises general fixes, or says PORT-010 keeps reporting the item.",
 "The item mentions both PORT-010 and a loopback TCP sibling but it is not possible to tell whether PORT-010 stops reporting the `#[cfg(unix)]` item."),
}
con={
"K0":base["contradicted"],
"K1":Q("Does `item.text` say that PORT-010 keeps reporting a `#[cfg(unix)]` transport or adapter item even though an unguarded loopback TCP sibling beside it serves Windows? Silence about this shape, or work on a different shape, is not a conflict. Judge conflict only.",
 "The item says PORT-010 keeps reporting, still reports, or is left unchanged for a `#[cfg(unix)]` transport or adapter item that has an unguarded loopback TCP sibling serving Windows.",
 "The item does not say PORT-010 keeps reporting that shape: it plans to stop reporting it, is silent about it, or handles a different shape.",
 "The item mentions the shape but it is not possible to tell whether PORT-010 will keep reporting it."),
"K2":Q("`anchor.statement` requires that PORT-010 not report a `#[cfg(unix)]` transport or adapter function or match arm when a loopback TCP sibling (enum variant or function) with no `#[cfg]` guard beside it serves Windows. Does `item.text` state a plan or outcome that cannot be reconciled with this, such as PORT-010 still reporting that shape, leaving PORT-010 unchanged for it, or relying on a consumer exclusion instead of the rule change? The requirement covers only unguarded siblings: still reporting a `#[cfg(unix)]` item whose loopback TCP sibling is itself `#[cfg]`-guarded is not a conflict. Silence about the shape, or work on a different shape, is not a conflict. Judge conflict only.",
 "The item says PORT-010 still reports, keeps reporting, or is left unchanged for a `#[cfg(unix)]` transport or adapter item beside an unguarded loopback TCP sibling that serves Windows, or says that shape is handled only by an exclusion.",
 "Nothing in the item conflicts: it plans to stop reporting the shape, is silent about it, handles a different shape, or keeps reporting only when the loopback TCP sibling is itself `#[cfg]`-guarded.",
 "The item mentions the shape and PORT-010 but it is not possible to tell whether PORT-010 will still report it."),
"K3":Q("Does `item.text` contain a statement that, if carried out, would leave PORT-010 reporting a `#[cfg(unix)]` transport or adapter function or match arm whose loopback TCP sibling has no `#[cfg]` guard and serves Windows? Read the item for its own statement about this case. An item that never mentions this case does not conflict. An item that keeps reporting only cases where the loopback TCP sibling carries a `#[cfg]` guard does not conflict. Judge conflict only.",
 "The item states that PORT-010 still reports, or will keep reporting, the `#[cfg(unix)]` item in the case where its loopback TCP sibling is unguarded and serves Windows.",
 "The item states no such thing: it stops reporting that case, never mentions it, handles some other shape, or keeps reporting only guarded-sibling cases.",
 "The item says something about this case that could be read either way."),
}
bundles={"B0":{"relevant":"R0","addressed":"A0","contradicted":"K0"},
 "B1":{"relevant":"R1","addressed":"A1","contradicted":"K1"},
 "B2":{"relevant":"R2","addressed":"A2","contradicted":"K2"},
 "B3":{"relevant":"R3","addressed":"A3","contradicted":"K3"}}
json.dump({"roles":{"relevant":rel,"addressed":add,"contradicted":con},"bundles":bundles},open('candidates.json','w'),indent=1,ensure_ascii=False)
txt=json.dumps({"roles":{"relevant":rel,"addressed":add,"contradicted":con}}).lower()
for p in ("confidence at least","confidence threshold","final verdict","role-to-verdict","classify the three roles","select that category confidently"):
    assert p not in txt,p
print("ok")
