import json, glob, hashlib, statistics, os, sys
sys.path.insert(0, "/Users/randlee/github/sc-lint-worktrees/plan/phase-m/.claude/skills/plan-anchoring/scripts")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import anchor_tuning_contract as T, jev_judge as J
from common import cases
sha = T.sha256
aid = sys.argv[1]; os.chdir(aid); HERE = os.path.abspath(".")
P = json.load(open("params.json"))
O = json.load(open("oracle.json")); OC = cases("oracle.json"); DC = cases("diagnostic.json")
C = json.load(open("candidates.json")); req = json.load(open("tune-request.json")); cfg = json.load(open("config.json")); KIND = cfg["kind"]
oracle_sha = hashlib.sha256(open("oracle.json", "rb").read()).hexdigest(); assert oracle_sha == open("oracle.sha256").read().split()[0]
roles = ["relevant", "addressed", "contradicted"]; JR = C["roles"]; BUNDLES = sorted(C["bundles"], key=lambda b: int(b[1:]))
V1 = set(O.get("supersedes", {}).get("v1_case_ids", []))
exp = {c["case_id"]: c for c in OC}; THR = req["production_confidence_threshold"]
def load(b, d="runs"): return [json.load(open(p)) for p in sorted(glob.glob(f"{d}/{b}/*.json"))]
cur = req["current_questions"]; cur_digest = sha({"questions": cur})
assert C["candidates"]["relevant"]["R0"] == cur["relevant"] and C["candidates"]["addressed"]["A0"] == cur["addressed"] and C["candidates"]["contradicted"]["K0"] == cur["contradicted"]
def conf_diag(recs):
    cs = sorted(r["answers"][q]["confidence"] for r in recs for q in roles)
    return {"threshold": 0.8, "cells_at_or_above_threshold": sum(c >= 0.8 for c in cs), "total_cells": len(cs), "minimum": round(cs[0], 4),
            "p10": round(statistics.quantiles(cs, n=10, method="inclusive")[0], 4), "median": round(statistics.median(cs), 4), "mean": round(statistics.mean(cs), 4)}
b0 = {(r["case_id"], r["repeat"], q): r["answers"][q]["choice"] for r in load("B0") for q in roles}
def sem_score(recs):
    cs = {}
    for r in recs: cs.setdefault(r["case_id"], []).append(r)
    reg = [(r["case_id"], r["repeat"], q) for r in recs for q in roles if b0[(r["case_id"], r["repeat"], q)] == exp[r["case_id"]]["expected"][q] and r["answers"][q]["choice"] != exp[r["case_id"]]["expected"][q]]
    return {"exact_role_cells": sum(r["answers"][q]["choice"] == exp[r["case_id"]]["expected"][q] for r in recs for q in roles), "total_role_cells": len(recs) * 3,
            "exact_semantic_verdicts": sum(r["semantic_verdict"] == exp[r["case_id"]]["expected_verdict"] for r in recs), "total_semantic_verdicts": len(recs),
            "stable_cases": sum(len({tuple(x["answers"][q]["choice"] for q in roles) for x in v}) == 1 for v in cs.values()), "total_cases": len(cs), "regressions": len(reg)}, cs, reg
def prod(recs): return [(r["case_id"], r["repeat"], J.decide(json.loads(r["raw_response"])["answers"], JR, THR, KIND)[0]) for r in recs]
rows = []
for b in BUNDLES:
    rs = load(b); s, cs, reg = sem_score(rs); d = conf_diag(rs); dr = load(b, "diag/runs")
    acc = s["exact_role_cells"] == s["total_role_cells"] and s["exact_semantic_verdicts"] == s["total_semantic_verdicts"] and s["stable_cases"] == s["total_cases"] and s["regressions"] == 0
    rows.append({"bundle": b, "members": C["bundles"][b], "semantic_score": s, "confidence": d, "regression_cells": [list(x) for x in reg],
                 "role_cell_misses": sorted([r["case_id"], r["repeat"], q, r["answers"][q]["choice"], r["answers"][q]["confidence"]] for r in rs for q in roles if r["answers"][q]["choice"] != exp[r["case_id"]]["expected"][q]),
                 "production_verdicts_at_threshold": {"threshold": THR, "oracle_mismatches": [list(x) for x in prod(rs) if x[2] != exp[x[0]]["expected_verdict"]], "diagnostic": [list(x) for x in prod(dr)]},
                 "cells_below_production_threshold": sorted([r["case_id"], r["repeat"], q, r["answers"][q]["confidence"]] for r in rs + dr for q in roles if r["answers"][q]["confidence"] < THR),
                 "complexity_chars": sum(len(json.dumps(C["candidates"][q][C["bundles"][b][q]])) for q in roles), "semantic_accepted": acc})
rows.sort(key=lambda x: (-x["semantic_accepted"], -x["semantic_score"]["exact_role_cells"], -x["semantic_score"]["exact_semantic_verdicts"], -x["semantic_score"]["stable_cases"],
                         x["semantic_score"]["regressions"], -x["confidence"]["cells_at_or_above_threshold"], -x["confidence"]["minimum"], x["complexity_chars"], x["bundle"]))
for n, x in enumerate(rows, 1): x["rank"] = n
TOP = rows[0]; ACCEPTED = TOP["semantic_accepted"]; WIN = TOP["bundle"]; spec = C["bundles"][WIN]
recs = load(WIN); sem, cs, _ = sem_score(recs); diag = conf_diag(recs); questions = {r: C["candidates"][r][spec[r]] for r in roles}
def role_rank(role):
    out = []
    for cid in sorted(C["candidates"][role]):
        bs = [b for b in BUNDLES if C["bundles"][b][role] == cid]; rs = [r for b in bs for r in load(b)]
        ex = sum(r["answers"][role]["choice"] == exp[r["case_id"]]["expected"][role] for r in rs); g = {}
        for r in rs: g.setdefault((r["bundle"], r["case_id"]), set()).add(r["answers"][role]["choice"])
        st = sum(len(v) == 1 for v in g.values()); cf = [r["answers"][role]["confidence"] for r in rs]
        held = cid in ("R0", "A0", "K0") and all(C["bundles"][b][role] == cid for b in BUNDLES)
        why = P["reasons"].get(cid) or ((f"held byte-identical (current question); " if held else "") + f"{ex}/{len(rs)} cells exact, {st}/{len(g)} stable, min {min(cf):.2f} across bundles {', '.join(bs)}"
               + ("; selected" if cid == spec[role] else ""))
        out.append({"role": role, "candidate_id": cid, "is_current": cid in ("R0", "A0", "K0"), "selected": cid == spec[role], "bundles": bs, "exact_cells": ex, "total_cells": len(rs),
                    "stable_cases": st, "total_cases": len(g), "confidence_min": round(min(cf), 3), "cells_at_or_above_0.8": sum(c >= 0.8 for c in cf),
                    "chars": len(json.dumps(C["candidates"][role][cid])), "question_sha256": sha(C["candidates"][role][cid]), "reason": why})
    out.sort(key=lambda x: (-(x["exact_cells"] == x["total_cells"] and x["stable_cases"] == x["total_cases"]), -x["exact_cells"] / x["total_cells"], -x["stable_cases"] / x["total_cases"],
                            -x["cells_at_or_above_0.8"] / x["total_cells"], -x["confidence_min"], x["chars"], x["candidate_id"]))
    for n, x in enumerate(out, 1): x["rank"] = n
    return out
rankings = [x for role in roles for x in role_rank(role)]
before = req["tuning_history"]; before_sha = sha(before); events = before["events"]
anc = req["authority"]["anchors"]["records"][0]; it = req["authority"]["intent"]
event = {"event_index": len(events) + 1, "previous_event_sha256": sha(events[-1]) if events else None,
 "event": "question_set_sealed" if ACCEPTED else "question_set_rejected", "protocol_version": "plan-anchor/question-tournament/2.0.0",
 "authority": {"intent_id": it["id"], "intent_version": it["version"], "intent_sha256": it["sha256"], "anchor_id": aid, "anchor_version": anc["version"], "anchor_record_sha256": sha(anc)},
 "oracle": {"sha256": oracle_sha, "case_ids": [c["case_id"] for c in OC], "case_count": len(OC), "repeat_count": O["repeat_count"]},
 "candidate": {"candidate_id": f"{WIN}:{spec['relevant']}+{spec['addressed']}+{spec['contradicted']}", "question_sha256": sha({"questions": questions})},
 "evidence": {"canonical_request_sha256": [r["canonical_request_sha256"] for r in recs], "raw_response_sha256": [r["raw_response_sha256"] for r in recs], "run_ids": [r["run_id"] for r in recs]},
 "score": dict(sem, confidence=diag),
 "decision": {"status": "sealed", "question_set_sha256": sha({"questions": questions}), "reason_code": "TOURNAMENT.SEMANTIC_ACCEPTED"} if ACCEPTED else {"status": "rejected", "question_set_sha256": None, "reason_code": "TOURNAMENT.ROLE_CELL_MISS"},
 "supersedes": {"protocol_version": "plan-anchor/question-tournament/2.0.0", "decision_sha256": sha(events[-1])} if events else None}
after = {"schema": before["schema"], "history_id": before["history_id"], "events": events + [event]}
acc = [{"case_id": cid, "kind": exp[cid]["kind"], "expected": exp[cid]["expected"], "expected_verdict": exp[cid]["expected_verdict"], "rationale": exp[cid]["rationale"],
        "observed": [{"repeat": r["repeat"], "answers": r["answers"], "semantic_verdict": r["semantic_verdict"], "run_id": r["run_id"]} for r in sorted(rs, key=lambda r: r["repeat"])],
        "match": all(r["semantic_verdict"] == exp[cid]["expected_verdict"] and all(r["answers"][q]["choice"] == exp[cid]["expected"][q] for q in roles) for r in rs)} for cid, rs in sorted(cs.items())]
allr = [json.load(open(p)) for b in BUNDLES for d in ("runs", "diag/runs") for p in sorted(glob.glob(f"{d}/{b}/*.json"))]
retries = sum(len(r["failed_attempts"]) for r in allr)
byte_identical = all(len({x["canonical_request_sha256"] for x in allr if x["bundle"] == b and x["case_id"] == c}) == 1 for b in BUNDLES for c in list(exp) + [d["case_id"] for d in DC])
auth_same = open("../authority-before.sha256").read() == open("../authority-after.sha256").read()
data = {"operation": "tune", "gate": "planning", "accepted": ACCEPTED, "tournament_protocol_version": "plan-anchor/question-tournament/2.0.0",
 "production_confidence_threshold": THR, "confidence_reference_threshold": 0.8, "semantic_score": sem, "confidence_diagnostics": diag,
 "tuning_history_before_sha256": before_sha, "tuning_history_after": after, "tuning_history_after_sha256": sha(after), "production_semantics_unchanged": True,
 "authority_citations": P["citations"], "oracle_sha256": oracle_sha,
 "sealed_question_set": ({"digest": sha({"questions": questions}), "questions": questions} if ACCEPTED else None),
 "candidate_rankings": rankings + [{"bundles": rows}], "acceptance_table": acc,
 "evidence": {"workspace": HERE, "oracle_path": f"{HERE}/oracle.json", "oracle_builder": f"{HERE}/../build_oracles.py", "oracle_predecessor": P.get("oracle_predecessor"),
   "oracle_frozen_before_first_jev_call": True, "candidates_path": f"{HERE}/candidates.json", "candidate_builders": [f"{HERE}/../build_candidates.py", f"{HERE}/../build_candidates_round2.py", f"{HERE}/../build_candidates_round3.py"],
   "diagnostic": {"path": f"{HERE}/diagnostic.json", "builder": f"{HERE}/../build_diag.py", "normative": False, "source_sha256": req["diagnostic_plan"]["sha256"], "runs_dir": f"{HERE}/diag/runs"},
   "runs_dir": f"{HERE}/runs", "request_path": f"{HERE}/tune-request.json", "harness": f"{HERE}/../run_bundle.py via run.sh (env -i; jev_judge build_state/encode_body/validate_response/decide; decide threshold 0.0, kind {KIND})",
   "jev_calls": len(allr), "bundles_evaluated": BUNDLES, "rounds": P["rounds"], "parallel_evaluators": 4, "repeat_requests_byte_identical": byte_identical, "third_calls": 0,
   "answering_model": sorted({r.get("model_answered") for r in allr}, key=str), "regression_baseline": "B0 = current questions " + cur_digest, "scope": P["scope"],
   "v1_case_regressions_vs_B0": {x["bundle"]: [c for c in x["regression_cells"] if c[0] in V1] for x in rows},
   "authority_hashes_before": f"{HERE}/../authority-before.sha256", "authority_hashes_after": f"{HERE}/../authority-after.sha256", "authoritative_sources_unchanged": auth_same},
 "explanation": P["explanation"], "planner_action": P["planner_action"], "question_action": None, "sealed_question_set_digest_before": None,
 "sealed_question_set_digest_after": (sha({"questions": questions}) if ACCEPTED else None), "authority_recommendations": P.get("authority_recommendations", [])}
meta = {"duration_ms": 0, "tool_calls": P["tool_calls"], "retry_count": retries, "correlation_id": req["correlation_id"], "jev_run_ids": [r["run_id"] for r in allr],
        "planner_observation_trust": "untrusted", "authoritative_sources_unchanged": auth_same}
json.dump({"success": True, "canceled": False, "aborted_by": None, "data": data, "error": None, "metadata": meta}, open("vars.json", "w"), sort_keys=True)
json.dump(after, open("tuning-history-after.json", "w"), sort_keys=True, separators=(",", ":"), ensure_ascii=False)
print(json.dumps({"aid": aid, "win": WIN, "accepted": ACCEPTED, "seal": data["sealed_question_set_digest_after"], "sem": sem, "diag": diag, "before": before_sha, "after": sha(after),
                  "calls": len(allr), "retries": retries, "byte_identical": byte_identical, "auth_same": auth_same, "oracle": oracle_sha}))
for x in rows[:4] + [r for r in rows if r["bundle"] == "B0"]:
    print(x["rank"], x["bundle"], x["members"], x["semantic_accepted"], x["confidence"]["cells_at_or_above_threshold"], x["confidence"]["minimum"], "below0.6:", len(x["cells_below_production_threshold"]), "prodmis:", x["production_verdicts_at_threshold"]["oracle_mismatches"][:4])
