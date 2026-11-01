import json, glob, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import cases
adir, ofile, rdir = sys.argv[1:4]; os.chdir(adir)
exp = {c["case_id"]: c for c in cases(ofile)}; roles = ["relevant", "addressed", "contradicted"]
cfgk = json.load(open("config.json"))["kind"]
for b in sys.argv[4:]:
    recs = [json.load(open(p)) for p in sorted(glob.glob(f"{rdir}/{b}/*.json"))]
    per = {q: [0, 0, 1.0] for q in roles}; verd = 0; cs = {}
    for r in recs:
        cs.setdefault(r["case_id"], []).append(r); verd += r["semantic_verdict"] == exp[r["case_id"]]["expected_verdict"]
        for q in roles:
            per[q][0] += r["answers"][q]["choice"] == exp[r["case_id"]]["expected"][q]; per[q][1] += 1; per[q][2] = min(per[q][2], r["answers"][q]["confidence"])
    st = sum(len({tuple(x["answers"][q]["choice"] for q in roles) for x in v}) == 1 for v in cs.values())
    print(b, {q: f"{v[0]}/{v[1]} min={v[2]:.2f}" for q, v in per.items()}, f"verdicts {verd}/{len(recs)} stable {st}/{len(cs)}")
    for cid, rs in sorted(cs.items()):
        e = exp[cid]; bad = any(r["semantic_verdict"] != e["expected_verdict"] or any(r["answers"][q]["choice"] != e["expected"][q] for q in roles) for r in rs)
        low = any(r["answers"][q]["confidence"] < 0.6 for r in rs for q in roles)
        if bad or low:
            print("  ", cid, [f"{q[0]}:{r['answers'][q]['choice'][0]}{r['answers'][q]['confidence']:.2f}" for r in rs for q in roles], [r["semantic_verdict"] for r in rs], "exp", e["expected_verdict"], {q: e["expected"][q][0] for q in roles}, "MISS" if bad else "LOW")
