import json, hashlib, sys, os
sys.path.insert(0, "/Users/randlee/github/sc-lint-worktrees/plan/phase-m/.claude/skills/plan-anchoring/scripts")
import anchor_tuning_contract as T
V = "/Users/randlee/github/sc-lint-worktrees/plan/phase-m/docs/plans/phase-m/v3"
base = json.load(open(f"{V}/anchor-runs/tuning/ANC-010/planning-v3/request.json"))
A = {r["id"]: r for r in json.load(open(f"{V}/anchors.json"))["anchors"]}
anchors_sha = hashlib.sha256(open(f"{V}/anchors.json", "rb").read()).hexdigest()
intent_raw = open(f"{V}/intent.md", "rb").read(); isha = hashlib.sha256(intent_raw).hexdigest()
intent = {"id": "phase-m-intent-v3", "version": isha[:8], "sha256": isha, "content": intent_raw.decode()}
F = "docs/plans/phase-m/v3/anchor-runs/plan-gate-review-fix1-20261031"
spec = {"ANC-010": ("lint-m-1", json.load(open(f"{V}/anchor-runs/tuning/ANC-010/planning-v3/tuning-history-after.json"))),
        "ANC-012": ("lint-m-3", {"schema": "plan-anchor/tuning-history/2", "history_id": "fix1-ANC-012-planning", "events": []}),
        "ANC-009": ("lint-m-4", {"schema": "plan-anchor/tuning-history/2", "history_id": "fix1-ANC-009-planning", "events": []})}
for aid, (bead, hist) in spec.items():
    C = json.load(open(f"{aid}/candidates.json"))
    r = {k: v for k, v in base.items()}
    r["correlation_id"] = f"fix1t-{aid}"
    r["authority"] = {"anchors": {"records": [A[aid]], "sha256": anchors_sha}, "intent": intent}
    r["current_questions"] = {"relevant": C["candidates"]["relevant"]["R0"], "addressed": C["candidates"]["addressed"]["A0"], "contradicted": C["candidates"]["contradicted"]["K0"]}
    r["tuning_history"] = hist
    item = f"/Users/randlee/github/sc-lint-worktrees/plan/phase-m/{F}/{bead}.item.txt"
    r["diagnostic_plan"] = {"read_only": True, "sha256": hashlib.sha256(open(item, "rb").read()).hexdigest(), "source_kind": "markdown", "source_ref": f"{F}/{bead}.item.txt"}
    json.dump(r, open(f"{aid}/tune-request.json", "w"), indent=1, sort_keys=True)
    print(aid, T.sha256(r), T.sha256({"questions": r["current_questions"]}), T.sha256(hist))
