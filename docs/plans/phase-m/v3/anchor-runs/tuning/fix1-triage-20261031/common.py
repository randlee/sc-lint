import json, hashlib
V = "/Users/randlee/github/sc-lint-worktrees/plan/phase-m/docs/plans/phase-m/v3"
def norm(c):
    if "case_id" in c: return c
    return {"case_id": c["id"], "kind": c.get("kind") or c["title"], "item": {"title": c["title"], "text": c["text"]},
            "expected": c["expected_roles"], "expected_verdict": c["expected_verdict"], "rationale": c["authority_rationale"]}
def cases(path): return [norm(c) for c in json.load(open(path))["cases"]]
