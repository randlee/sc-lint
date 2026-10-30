import json,sys,statistics,collections
sys.path.insert(0,'/Users/randlee/github/sc-lint-worktrees/plan/phase-m/.claude/skills/plan-anchoring/scripts')
import jev_judge as J
oracle=json.load(open('oracle.json')); exp={c['case_id']:c for c in oracle['cases']}
ROLES={"relevance":"relevant","compliance":"addressed","conflict":"contradicted"}
def analyze(res):
    by=collections.defaultdict(lambda:collections.defaultdict(list))
    for r in res:
        a=r['attempts'][-1]; by[r['bundle']][r['case_id']].append(a)
    out={}
    for b,cases in by.items():
        role_ok=collections.Counter(); role_tot=collections.Counter(); confs=[]; rows=[]
        exv=0;tv=0;stable=0;erc=0;trc=0
        for cid,atts in cases.items():
            e=exp[cid]; tuples=[]
            for a in atts:
                ans=a['answers']; t=tuple(J.answer_choice(ans[q])[0] for q in ("relevant","addressed","contradicted")); tuples.append(t)
                for q in ("relevant","addressed","contradicted"):
                    ch,cf=J.answer_choice(ans[q]); confs.append(cf); role_tot[q]+=1; trc+=1
                    if ch==e['expected'][q]: role_ok[q]+=1; erc+=1
                v=J.decide(ans,ROLES,0.0,"directive")[0]; tv+=1; exv+= v==e['expected_verdict']
                rows.append((cid,t,[round(J.answer_choice(ans[q])[1],2) for q in ("relevant","addressed","contradicted")],v,e['expected_verdict']))
            stable+= len(set(tuples))==1
        out[b]=dict(role_ok=dict(role_ok),erc=erc,trc=trc,exv=exv,tv=tv,stable=stable,cases=len(cases),confs=confs,rows=rows)
    return out
if __name__=="__main__":
    res=[r for f in sys.argv[1:] for r in json.load(open(f))]
    for b,o in sorted(analyze(res).items()):
        print(f"== {b} cells {o['erc']}/{o['trc']} verdicts {o['exv']}/{o['tv']} stable {o['stable']}/{o['cases']} per-role {o['role_ok']} minconf {min(o['confs']):.2f} >=0.8 {sum(c>=0.8 for c in o['confs'])}")
        for row in o['rows']:
            flag='' if row[3]==row[4] else '  <-- MISS'
            print('  ',row[0][:28].ljust(28),row[1],row[2],row[3],flag)
