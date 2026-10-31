import json, sys, os, hashlib, urllib.request, urllib.error, time
sys.path.insert(0, "/Users/randlee/github/sc-lint-worktrees/plan/phase-m/.claude/skills/plan-anchoring/scripts")
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import jev_judge as J
from common import cases, V
adir, bundle_id, ofile, odir = sys.argv[1:5]
os.chdir(adir)
cfg = json.load(open("config.json")); CORR = cfg["correlation_id"]
C = json.load(open("candidates.json")); spec = C["bundles"][bundle_id]
questions = {role: C["candidates"][role][cid] for role, cid in spec.items()}
anchor = {r["id"]: r for r in json.load(open(f"{V}/anchors.json"))["anchors"]}[cfg["anchor_id"]]
key = os.environ["TYPESAFE_API_KEY"].strip()
out = f"{odir}/{bundle_id}"; os.makedirs(out, exist_ok=True)
def post(data):
    r = urllib.request.Request(J.JEV_URL, data=data, method="POST", headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json", "User-Agent": "plan-anchoring/anchor-tuning"})
    with urllib.request.urlopen(r, timeout=120) as resp: return resp.read(J.MAX_RESPONSE_BYTES + 1)
retries = 0
for case in cases(ofile):
    for rep in (1, 2):
        path = f"{out}/{case['case_id']}-r{rep}.json"
        if os.path.exists(path): continue
        item = {"id": f"{CORR}/{case['case_id']}", "title": case["item"]["title"], "text": case["item"]["text"]}
        data = J.encode_body({"model": J.DEFAULT_MODEL, "state": J.build_state(anchor, item), "questions": questions})
        attempts = []
        for attempt in (1, 2):
            try:
                raw = post(data); parsed = json.loads(raw.decode()); J.validate_response(parsed, questions); break
            except (urllib.error.URLError, OSError, ValueError, J.JudgeError) as e:
                code = getattr(e, "code", type(e).__name__); attempts.append({"attempt": attempt, "error": str(code)[:200]})
                if attempt == 2: raise SystemExit(f"EXECUTION.JEV {bundle_id} {case['case_id']} r{rep}: {code}")
                retries += 1; time.sleep(2)
        ans = parsed["answers"]; verdict, _ = J.decide(ans, C["roles"], 0.0, cfg["kind"])
        rec = {"run_id": f"{CORR}/{bundle_id}/{case['case_id']}/r{rep}", "bundle": bundle_id, "spec": spec, "case_id": case["case_id"], "repeat": rep,
               "canonical_request": data.decode(), "canonical_request_sha256": hashlib.sha256(data).hexdigest(),
               "raw_response": raw.decode(), "raw_response_sha256": hashlib.sha256(raw).hexdigest(), "failed_attempts": attempts,
               "answers": {q: {"choice": J.answer_choice(a)[0], "confidence": J.answer_choice(a)[1]} for q, a in ans.items()},
               "semantic_verdict": verdict, "model_answered": parsed.get("model")}
        json.dump(rec, open(path, "w"), sort_keys=True, indent=1)
print(json.dumps({"anchor": cfg["anchor_id"], "bundle": bundle_id, "file": ofile, "retries": retries}))
