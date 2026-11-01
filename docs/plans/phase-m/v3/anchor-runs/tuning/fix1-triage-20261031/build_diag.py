import json, hashlib, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from common import V
F = f"{V}/anchor-runs/plan-gate-review-fix1-20261031"
spec = {"ANC-010": ("lint-m-1", ("no", "no", "no"), "not_applicable", "m-1 changes S3/S6 recognition and names m-4's allowance only as the route for other shapes and restricted pub items; it does not add or change allowance handling (ANC-010 v3 scope)."),
        "ANC-012": ("lint-m-3", ("yes", "yes", "no"), "satisfied", "m-3 adds lint-step env/cwd (the pass-fixture subject) and states setup/runner stay rejected and no setup-hook or interpreter-selection key is added."),
        "ANC-009": ("lint-m-4", ("yes", "yes", "no"), "satisfied", "m-4 adds a reasoned single-item PORT-010 allowance with no cascade and no repo-wide switch, and keeps unannotated siblings reported; INTENT: other shapes use reasoned site allowances, unallowed real gaps still report.")}
for aid, (bead, (r, a, c), v, why) in spec.items():
    raw = open(f"{F}/{bead}.item.txt", "rb").read(); text = raw.decode()
    title = text.splitlines()[0].lstrip("# ").strip()
    ceremony = text.replace("## Acceptance criteria", "### Process\nDaily status table, RACI matrix, two-reviewer sign-off checklist and a retrospective note before close.\n\n## Acceptance criteria", 1)
    assert ceremony != text
    d = {"schema": "plan-anchor/tuning-diagnostic/1", "normative": False, "source_ref": f"{F}/{bead}.item.txt", "source_sha256": hashlib.sha256(raw).hexdigest(), "repeat_count": 2,
         "cases": [{"case_id": cid, "kind": k, "item": {"title": title, "text": t}, "expected": {"relevant": r, "addressed": a, "contradicted": c}, "expected_verdict": v, "rationale": why}
                   for cid, k, t in (("D1", "diagnostic_as_is", text), ("D2", "diagnostic_positive_ceremony", ceremony))]}
    b = json.dumps(d, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode(); open(f"{aid}/diagnostic.json", "wb").write(b)
    print(aid, bead, hashlib.sha256(b).hexdigest())
