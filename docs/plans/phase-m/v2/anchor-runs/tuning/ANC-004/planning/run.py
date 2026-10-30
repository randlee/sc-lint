import json,hashlib,sys,os,urllib.request,urllib.error,pathlib,concurrent.futures as cf
sys.path.insert(0,'/Users/randlee/github/sc-lint-worktrees/plan/phase-m/.claude/skills/plan-anchoring/scripts')
import jev_judge as J
assert not [k for k in os.environ if k.startswith('PLAN_ANCHOR')]
KEY=os.environ['TYPESAFE_API_KEY'].strip()
oracle=json.load(open('oracle.json')); cands=json.load(open('candidates.json')); req=json.load(open('request.json'))
anc=req['authority']['anchors']['records'][0]
H=lambda b:hashlib.sha256(b).hexdigest()
out=pathlib.Path('evidence'); out.mkdir(exist_ok=True)
def bundle_q(spec):
    return {r:cands['roles'][r][cid] for r,cid in spec.items()}
def call(run_id,body,attempt):
    data=J.encode_body(body)
    r=urllib.request.Request(J.JEV_URL,data=data,method="POST",headers={"Authorization":f"Bearer {KEY}","Content-Type":"application/json","User-Agent":"plan-anchoring/anchor-tuning"})
    rec={"run_id":run_id,"attempt":attempt,"request_sha256":H(data)}
    (out/f"{run_id.replace('/','__')}.a{attempt}.request.json").write_bytes(data)
    try:
        with urllib.request.urlopen(r,timeout=90) as resp: raw=resp.read()
        (out/f"{run_id.replace('/','__')}.a{attempt}.response.json").write_bytes(raw)
        rec["response_sha256"]=H(raw)
        parsed=J.validate_response(json.loads(raw),body["questions"])
        rec["answers"]=parsed["answers"]; rec["model"]=parsed.get("model"); rec["ok"]=True
    except Exception as e:
        rec["ok"]=False; rec["error"]=f"{type(e).__name__}: {getattr(e,'code','')} {str(e)[:200]}"
    return rec
def job(bid,spec,case,n):
    run_id=f"v2-ANC-004-planning/{bid}/{case['case_id']}/r{n}"
    body={"model":"jev-latest","state":J.build_state(anc,case["item"]),"questions":bundle_q(spec)}
    recs=[call(run_id,body,1)]
    if not recs[0]["ok"]: recs.append(call(run_id,body,2))
    return {"bundle":bid,"case_id":case["case_id"],"repeat":n,"run_id":run_id,"attempts":recs}
def main(bids,label):
    specs=cands['bundles'] if not os.path.exists('bundles_extra.json') else {**cands['bundles'],**json.load(open('bundles_extra.json'))}
    jobs=[(b,specs[b],c,n) for b in bids for c in oracle['cases'] for n in (1,2)]
    with cf.ThreadPoolExecutor(4) as ex: res=list(ex.map(lambda a:job(*a),jobs))
    json.dump(res,open(f'results-{label}.json','w'),indent=1)
    print(len(res),'jobs; failures',sum(1 for r in res if not r['attempts'][-1]['ok']),'retries',sum(len(r['attempts'])-1 for r in res))
main(sys.argv[2].split(','),sys.argv[1])
