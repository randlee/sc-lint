import json,hashlib
req=json.load(open('request.json'))
anc=req['authority']['anchors']['records'][0]
F=anc['fixtures']
def case(cid,kind,title,text,rel,add,con,verdict,why):
    return {"case_id":cid,"kind":kind,"item":{"id":cid,"title":title,"text":text},
            "expected":{"relevant":rel,"addressed":add,"contradicted":con},"expected_verdict":verdict,"rationale":why}
cases=[
 case("C01-fixture-pass","approved_fixture_pass","fixture",F['pass'][0],"yes","yes","no","satisfied",
  "Approved pass fixture: PORT-010 stops reporting a #[cfg(unix)] match arm beside an un-gated loopback TCP variant used on Windows; this is ANC-004's directive verbatim in outcome form."),
 case("C02-fixture-fail","approved_fixture_fail","fixture",F['fail'][0],"yes","no","yes","contradicted",
  "Approved fail fixture: PORT-010 still reports exactly the S4 shape ANC-004 says must not be reported. The directive is absent (addressed=no) and the stated outcome cannot be reconciled with it (contradicted=yes); the role map yields contradicted, within the fixture's allowed set."),
 case("C03-compliant-paraphrase-fn","compliant_paraphrase","Sprint MS4 (#786): loopback TCP adapter sibling",
  "Extend PORT-010 in crates/sc-lint-portability so that a #[cfg(unix)] adapter function such as connect_unix_socket is treated as covered, and produces no finding, when the same module also defines a connect_loopback_tcp function with no #[cfg] attribute that the Windows build calls instead. Add a regression fixture for this shape that expects zero findings.",
  "yes","yes","no","satisfied",
  "Different vocabulary, function form ('enum variant or function' in the statement): no finding for a cfg(unix) adapter fn beside an unguarded loopback TCP fn used by Windows. Directive present, no conflict."),
 case("C04-boundary-guarded-sibling","boundary","Sprint MS4 (#786): S4 loopback TCP sibling",
  "Teach PORT-010 to stop reporting a #[cfg(unix)] transport match arm when the enum it matches on has a loopback TCP variant with no #[cfg] guard that Windows uses. Keep the condition strict: if the loopback TCP variant is itself marked #[cfg(unix)], PORT-010 still reports the arm, because Windows then has no counterpart. Fixtures cover both cases.",
  "yes","yes","no","satisfied",
  "Boundary of 'with no #[cfg] guard': the statement covers only unguarded siblings, so continuing to report when the sibling is itself cfg(unix)-gated is consistent with ANC-004 (and with INTENT: real parity gaps are still reported). Directive present; the narrower still-reports clause is not a conflict."),
 case("C05-silent-other-shape","silence","Sprint MS2 (#720): same-name cfg(windows) sibling",
  "Change PORT-010 in crates/sc-lint-portability so it no longer reports a #[cfg(unix)] function when a function with the same name and a #[cfg(windows)] attribute exists in the same module. Add regression fixtures for the same-name sibling shape in function, impl and trait scope.",
  "yes","no","no","not_addressed",
  "Relevant per anchor.scope (changes when PORT-010 stops reporting a cfg(unix) item) but silent about the loopback TCP sibling shape. Silence is not conflict."),
 case("C06-topical-only","topical_only","Sprint MS4 (#786): PORT-010 transport findings",
  "Review the remaining PORT-010 findings that atm-core reports on its transport and adapter code, and adjust the PORT-010 rule in crates/sc-lint-portability where those findings are false positives.",
  "yes","no","no","not_addressed",
  "Mentions PORT-010, transport/adapter code and #786 but names neither the loopback TCP sibling nor the absent-guard condition; a generic promise to fix false positives is not the directive's specific content. No conflict."),
 case("C07-boundary-other-sibling","boundary","Sprint MS5: named-pipe sibling",
  "Make PORT-010 stop reporting a #[cfg(unix)] Unix-socket match arm when the same enum has a #[cfg(windows)] NamedPipe variant beside it. Add a fixture for the named-pipe sibling.",
  "yes","no","no","not_addressed",
  "Same kind of change (cfg(unix) match arm suppression) but the sibling is a cfg(windows)-gated named pipe, not an unguarded loopback TCP sibling. ANC-004's shape is absent; nothing rules it out."),
 case("C08-conflict","conflict","Sprint MS4 (#786): leave transport arms",
  "Leave PORT-010 unchanged for #[cfg(unix)] transport match arms that sit beside an un-gated loopback TCP enum variant; PORT-010 keeps reporting them, and atm-core keeps its lint exclusion for those paths.",
  "yes","no","yes","contradicted",
  "Directly states the opposite of ANC-004 for the S4 shape (keeps reporting) and relies on an exclusion, which INTENT puts out of scope."),
 case("C09-irrelevant-feature","irrelevant","Sprint MS7 (#136): source-only ownership scope",
  "Add an opt-in source-only scope to global_dependency_ownership so that dependency ownership is checked only for source files when a consumer enables it; checking all files remains the default.",
  "no","no","no","not_applicable",
  "Different feature (manifest dependency ownership); does not change when PORT-010 reports any item."),
 case("C10-irrelevant-same-crate","irrelevant","Sprint MS9: PORT-011 path separators",
  "Add a new portability rule PORT-011 in crates/sc-lint-portability that reports hard-coded '/' path separators in string literals passed to std::fs functions.",
  "no","no","no","not_applicable",
  "Same crate, different rule: PORT-011 does not change when PORT-010 reports or stops reporting a cfg(unix) item, so it is outside anchor.scope."),
]
oracle={"schema":"plan-anchor/tuning-oracle/2","correlation_id":"v2-ANC-004-planning","gate":"planning",
 "anchor_id":"ANC-004","anchor_version":3,"intent_id":"phase-m-intent","intent_version":"717d363a",
 "role_map":{"relevance":"relevant","compliance":"addressed","conflict":"conflict" and "contradicted"},"kind":"directive",
 "verdict_threshold":0.0,"repeat_count":2,"cases":cases}
open('oracle.json','w').write(json.dumps(oracle,indent=1,ensure_ascii=False))
c=lambda v:hashlib.sha256(json.dumps(v,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode()).hexdigest()
print(c(oracle))
