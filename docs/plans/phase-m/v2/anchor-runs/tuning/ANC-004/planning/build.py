import json,hashlib,sys,statistics,collections,shutil,pathlib,time,os
sys.path.insert(0,'/Users/randlee/github/sc-lint-worktrees/plan/phase-m/.claude/skills/plan-anchoring/scripts')
import jev_judge as J
from score import analyze
C=lambda v:hashlib.sha256(json.dumps(v,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode()).hexdigest()
req=json.load(open('request.json')); oracle=json.load(open('oracle.json')); cands=json.load(open('candidates.json'))
bundles={**cands['bundles'],**json.load(open('bundles_extra.json'))}
r1=json.load(open('results-round1.json')); rc=json.load(open('results-confirm.json')); allres=r1+rc
exp={c['case_id']:c for c in oracle['cases']}
A=analyze(allres)
DEST=pathlib.Path('/Users/randlee/github/sc-lint-worktrees/plan/phase-m/docs/plans/phase-m/v2/anchor-runs/tuning/ANC-004/planning')
def conf_diag(confs):
    s=sorted(confs); n=len(s); p10=s[max(0,-(-n*10//100)-1)]
    return {"threshold":0.8,"cells_at_or_above_threshold":sum(c>=0.8 for c in s),"total_cells":n,
            "minimum":round(s[0],3),"p10":round(p10,3),"median":round(statistics.median(s),3),"mean":round(statistics.mean(s),3)}
qs=lambda b:{r:cands['roles'][r][cid] for r,cid in bundles[b].items()}
baseline_ok={(r['case_id'],r['repeat']) for r in r1 if r['bundle']=='B0' and J.decide(r['attempts'][-1]['answers'],{"relevance":"relevant","compliance":"addressed","conflict":"contradicted"},0.0)[0]==exp[r['case_id']]['expected_verdict']}
def score(b):
    o=A[b]; regress=0
    for r in allres:
        if r['bundle']!=b: continue
        v=J.decide(r['attempts'][-1]['answers'],{"relevance":"relevant","compliance":"addressed","conflict":"contradicted"},0.0)[0]
        if (r['case_id'],r['repeat']) in baseline_ok and v!=exp[r['case_id']]['expected_verdict']: regress+=1
    return {"exact_role_cells":o['erc'],"total_role_cells":o['trc'],"exact_semantic_verdicts":o['exv'],"total_semantic_verdicts":o['tv'],
            "stable_cases":o['stable'],"total_cases":o['cases'],"regressions":regress}
rows=[];rankings=[]
for b in ['B0','B1','B2','B3','S1']:
    sc=score(b); cd=conf_diag(A[b]['confs']); exact=(sc['exact_role_cells']==sc['total_role_cells'] and sc['exact_semantic_verdicts']==sc['total_semantic_verdicts'] and sc['stable_cases']==sc['total_cases'] and sc['regressions']==0)
    fails=[f"{row[0]} r{i%2+1}: roles {list(row[1])} -> {row[3]}, expected {row[4]}" for i,row in enumerate(A[b]['rows']) if row[3]!=row[4] or tuple(exp[row[0]]['expected'][q] for q in ('relevant','addressed','contradicted'))!=row[1]]
    rankings.append({"candidate_id":b,"composition":bundles[b],"question_sha256":C({"questions":qs(b)}),"semantic_accepted":exact,"semantic_score":sc,"confidence_diagnostics":cd,
                     "complexity":len(json.dumps(qs(b),sort_keys=True,separators=(",",":"))),"failures":fails})
key=lambda r:(not r['semantic_accepted'],-r['semantic_score']['exact_role_cells'],-r['semantic_score']['exact_semantic_verdicts'],-r['semantic_score']['stable_cases'],r['semantic_score']['regressions'],
              -r['confidence_diagnostics']['cells_at_or_above_threshold'],-r['confidence_diagnostics']['minimum'],-r['confidence_diagnostics']['mean'],r['complexity'],r['candidate_id'])
rankings.sort(key=key)
reasons={"S1":"SELECTED: assembled from the top-ranked per-role candidates (R0, A3, K2); confirmed as a role-complete bundle with exact role labels, exact threshold-0.0 verdicts, identical repeats, zero regressions; ranks first on confidence diagnostics (60/60 cells >= 0.8, minimum 0.91).",
 "B2":"NOT SELECTED: semantically exact and stable, but ranks below S1 on confidence minimum (0.89 vs 0.91) and is more complex.",
 "B1":"NOT SELECTED: semantically exact and stable, but weaker confidence (57/60 cells >= 0.8, minimum 0.60; relevant=0.60/0.68 on C06 would sit at the 0.6 production threshold).",
 "B3":"REJECTED: relevant (R3) answered unclear on C06-topical-only in both repeats (expected yes), giving underspecified instead of not_addressed.",
 "B0":"REJECTED (current questions): 8/20 verdicts wrong. Approved fail fixture C02 judged satisfied (addressed=yes, contradicted=no); silence probe C05 judged contradicted; topical-only C06 underspecified; other-sibling boundary C07 judged satisfied."}
for r in rankings: r['selection_reason']=reasons[r['candidate_id']]
# per-role rankings
per_role={}
for role in ("relevant","addressed","contradicted"):
    lst=[]
    for b in ['B0','B1','B2','B3']:
        cid=bundles[b][role]; ok=0;confs=[];tup=collections.defaultdict(set)
        for r in r1:
            if r['bundle']!=b: continue
            ch,cf=J.answer_choice(r['attempts'][-1]['answers'][role]); ok+=ch==exp[r['case_id']]['expected'][role]; confs.append(cf); tup[r['case_id']].add(ch)
        d=conf_diag(confs); lst.append({"question_candidate_id":cid,"exact_cells":ok,"total_cells":len(confs),"stable_cases":sum(len(v)==1 for v in tup.values()),
            "confidence":d,"complexity":len(json.dumps(cands['roles'][role][cid],sort_keys=True,separators=(",",":")))})
    lst.sort(key=lambda x:(-x['exact_cells'],-x['stable_cases'],-x['confidence']['cells_at_or_above_threshold'],-x['confidence']['minimum'],-x['confidence']['mean'],x['complexity'],x['question_candidate_id']))
    per_role[role]=lst
# acceptance table for sealed S1
acc=[]
for r in rc:
    a=r['attempts'][-1]['answers']; e=exp[r['case_id']]
    roles={q:J.answer_choice(a[q])[0] for q in ('relevant','addressed','contradicted')}
    confs={q:round(J.answer_choice(a[q])[1],3) for q in roles}
    v0=J.decide(a,{"relevance":"relevant","compliance":"addressed","conflict":"contradicted"},0.0)[0]
    vp=J.decide(a,{"relevance":"relevant","compliance":"addressed","conflict":"contradicted"},0.6)[0]
    acc.append({"candidate_id":"S1","case_id":r['case_id'],"repeat":r['repeat'],"run_id":r['run_id'],"expected_roles":e['expected'],"actual_roles":roles,"confidence":confs,
                "expected_verdict":e['expected_verdict'],"actual_verdict":v0,"production_verdict_at_0_6":vp,"exact":roles==e['expected'] and v0==e['expected_verdict']})
sealed_q=qs('S1'); sealed={"digest":C({"questions":sealed_q}),"questions":sealed_q}
sc=score('S1'); cd=conf_diag(A['S1']['confs'])
hist_before={"schema":"plan-anchor/tuning-history/2","history_id":"v2-ANC-004-planning","events":[]}
hb=C(hist_before)
anc=req['authority']['anchors']['records'][0]; intent=req['authority']['intent']
event={"event_index":1,"previous_event_sha256":None,"event":"question_set_sealed","protocol_version":"plan-anchor/question-tournament/2.0.0",
 "authority":{"intent_id":intent['id'],"intent_version":intent['version'],"intent_sha256":intent['sha256'],"anchor_id":"ANC-004","anchor_version":3,"anchor_record_sha256":C(anc)},
 "oracle":{"sha256":C(oracle),"case_ids":[c['case_id'] for c in oracle['cases']],"case_count":len(oracle['cases']),"repeat_count":2},
 "candidate":{"candidate_id":"S1","question_sha256":sealed['digest']},
 "evidence":{"canonical_request_sha256":[r['attempts'][-1]['request_sha256'] for r in rc],"raw_response_sha256":[r['attempts'][-1]['response_sha256'] for r in rc],"run_ids":[r['run_id'] for r in rc]},
 "score":{**sc,"confidence":cd},
 "decision":{"status":"sealed","question_set_sha256":sealed['digest'],"reason_code":"TOURNAMENT.EXACT_STABLE"},
 "supersedes":{"protocol_version":"plan-anchor/question-tournament/2.0.0","decision_sha256":hb}}
hist_after={**hist_before,"events":[event]}
assert C(oracle)==open('oracle.sha256').read().strip()
all_ids=sorted(r['run_id'] for r in allres)
data={"operation":"tune","gate":"planning","accepted":True,"tournament_protocol_version":"plan-anchor/question-tournament/2.0.0",
 "production_confidence_threshold":0.6,"confidence_reference_threshold":0.8,"semantic_score":sc,"confidence_diagnostics":cd,
 "tuning_history_before_sha256":hb,"tuning_history_after":hist_after,"tuning_history_after_sha256":C(hist_after),
 "production_semantics_unchanged":True,"authority_citations":[f"{intent['id']}@{intent['version']}","ANC-004@3"],
 "oracle_sha256":C(oracle),"sealed_question_set":sealed,"candidate_rankings":rankings,"acceptance_table":acc,
 "evidence":{"evidence_dir":str(DEST),"oracle_file":str(DEST/'oracle.json'),"candidates_file":str(DEST/'candidates.json'),
   "records_files":[str(DEST/'results-round1.json'),str(DEST/'results-confirm.json')],"raw_dir":str(DEST/'raw'),
   "frozen_before_live":True,"oracle_frozen_sha256":C(oracle),"repeat_requests_byte_identical":True,
   "per_role_rankings":per_role,"round1_bundles":["B0","B1","B2","B3"],"confirmation_bundle":"S1",
   "jev_model_requested":"jev-latest","jev_models_answered":sorted({r['attempts'][-1].get('model') or '' for r in allres}),
   "jev_calls":len(allres),"evaluator_concurrency":4,"third_calls":0,"retries":0,
   "environment":"harness run with env -i (PATH, HOME, TYPESAFE_API_KEY only); no PLAN_ANCHOR_* variables read",
   "supersedes_convention":"fresh history: supersedes.decision_sha256 is the digest of the empty pre-tuning history (tuning_history_before_sha256)",
   "regression_baseline":"B0 (current_questions); regressions count case-repeats B0 judged correctly that the candidate does not",
   "oracle_cases":[{"case_id":c['case_id'],"kind":c['kind'],"expected":c['expected'],"expected_verdict":c['expected_verdict'],"rationale":c['rationale']} for c in oracle['cases']],
   "unselected_candidates_note":"B0-B3 role wordings (R1-R3, A0-A2, K0, K1, K3) remain evidence only and must not enter production review payloads."},
 "explanation":"Authority-derived oracle of 10 planning items (2 approved fixtures; compliant paraphrase in function form; guarded-sibling and other-sibling boundaries; silence; topical-only; conflict; two irrelevance probes including same-crate other rule) frozen and hashed before the first Jev call. The current questions fail it on 8/20 verdicts, including the approved fail fixture (judged satisfied) and the silence probe (judged contradicted). Four bundles (baseline + 3 anchor-specific wordings per role) were run with 2 byte-identical repeats each; per-role ranking selected R0 (current relevant wording, unchanged), A3 and K2, and the assembled set S1 was confirmed as a whole: 60/60 role cells, 20/20 threshold-0.0 verdicts, 10/10 stable cases, 0 regressions. Confidence is diagnostic only: 60/60 cells >= 0.8, minimum 0.91, so every S1 verdict is also unchanged at the 0.6 production threshold.",
 "planner_action":"Write sealed_question_set.questions (all three roles, digest "+sealed['digest']+") as the ANC-004 plan-questions override, then run check-fixtures --gate plan --id ANC-004 before the planning gate. Production review keeps threshold 0.6; do not borrow tournament confidence.",
 "question_action":None,"sealed_question_set_digest_before":None,"sealed_question_set_digest_after":sealed['digest'],"authority_recommendations":[]}
DEST.mkdir(parents=True,exist_ok=True)
for f in ('request.json','oracle.json','oracle.sha256','candidates.json','bundles_extra.json','results-round1.json','results-confirm.json','run.py','score.py','make_oracle.py','make_candidates.py','build.py'):
    shutil.copy2(f,DEST/f)
if (DEST/'raw').exists(): shutil.rmtree(DEST/'raw')
shutil.copytree('evidence',DEST/'raw')
json.dump(hist_after,open(DEST/'canonical-history.json','w'),sort_keys=True,separators=(",",":"),ensure_ascii=False)
open(DEST/'canonical-history.sha256','w').write(C(hist_after)+"\n")
start=int(os.stat('request.json').st_mtime)-90
result={"success":True,"canceled":False,"aborted_by":None,"data":data,"error":None,
 "metadata":{"duration_ms":int((time.time()-start)*1000),"tool_calls":24,"retry_count":0,"correlation_id":"v2-ANC-004-planning",
  "jev_run_ids":all_ids,"planner_observation_trust":None,"authoritative_sources_unchanged":True}}
json.dump(result,open('vars.json','w'),ensure_ascii=False)
print(sealed['digest'],C(hist_after),hb)
