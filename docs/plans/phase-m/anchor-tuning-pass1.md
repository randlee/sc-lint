# Phase M independent question tuning — pass 1

Task `lint-h3ro`. **15 eligible anchors × planning/development = 30 accepted semantic tournaments.** ANC-018/020/021 remain pending exception-path definition, intentionally untuned. This is not an anchor planning-gate or development-gate PASS.

Rand explicitly directed proceeding with the fixture packet at `023ec9d2`: “Rand directed proceeding; fixtures reviewable after the fact.” The eight issue-derived records were re-registered without wording/fixture changes at `19ea7566` (ANC-026..033). All fifteen input statements, quotes and fixtures were checked against that authorized packet; only the approved ID mapping differs.

Independent Sol evaluators froze per-anchor/gate oracles before Jev answers, considered at least three complete candidate bundles with two byte-identical repeats per case, preserved rejected rounds, and changed questions only. The parent independently validated request/result schemas, relational contracts, immutable oracle digests, approved fixture preservation, selected raw role answers/verdicts and repeat body hashes. No locked statement, quote, scope, fixture, version or lock state changed. Overrides were copied exactly from the validated sealed question sets into `anchors.json`.

## Per-anchor acceptance

Exact role-cell scores below are the selected bundle across frozen cases and repeats. Full oracle hashes, sealed question hashes, acceptance tables and call identifiers are linked per record in [summary.json](anchor-runs/tuning-pass-1/summary.json). Each gate folder contains `oracle.json`, `oracle.sha256`, `acceptance-table.json`, `run-ids.json`, `canonical-history.json`, the rendered result, parent validation, runner attestation and a SHA-256-bound raw archive.

| Anchor | Planning exact cells | Development exact cells | Low-confidence selected calls (plan/dev) | Evidence |
| --- | --- | --- | --- | --- |
| ANC-014 | 54/54 | 88/88 | 0/0 | [plan](anchor-runs/tuning-pass-1/ANC-014/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-014/development/result.json) |
| ANC-016 | 54/54 | 96/96 | 0/0 | [plan](anchor-runs/tuning-pass-1/ANC-016/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-016/development/result.json) |
| ANC-019 | 54/54 | 88/88 | 0/0 | [plan](anchor-runs/tuning-pass-1/ANC-019/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-019/development/result.json) |
| ANC-022 | 54/54 | 88/88 | 0/0 | [plan](anchor-runs/tuning-pass-1/ANC-022/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-022/development/result.json) |
| ANC-023 | 66/66 | 104/104 | 4/4 | [plan](anchor-runs/tuning-pass-1/ANC-023/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-023/development/result.json) |
| ANC-024 | 60/60 | 104/104 | 0/4 | [plan](anchor-runs/tuning-pass-1/ANC-024/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-024/development/result.json) |
| ANC-025 | 60/60 | 96/96 | 2/2 | [plan](anchor-runs/tuning-pass-1/ANC-025/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-025/development/result.json) |
| ANC-026 | 60/60 | 104/104 | 0/2 | [plan](anchor-runs/tuning-pass-1/ANC-026/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-026/development/result.json) |
| ANC-027 | 54/54 | 80/80 | 0/2 | [plan](anchor-runs/tuning-pass-1/ANC-027/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-027/development/result.json) |
| ANC-028 | 60/60 | 88/88 | 1/0 | [plan](anchor-runs/tuning-pass-1/ANC-028/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-028/development/result.json) |
| ANC-029 | 60/60 | 88/88 | 2/0 | [plan](anchor-runs/tuning-pass-1/ANC-029/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-029/development/result.json) |
| ANC-030 | 54/54 | 88/88 | 0/0 | [plan](anchor-runs/tuning-pass-1/ANC-030/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-030/development/result.json) |
| ANC-031 | 54/54 | 96/96 | 2/0 | [plan](anchor-runs/tuning-pass-1/ANC-031/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-031/development/result.json) |
| ANC-032 | 66/66 | 104/104 | 0/2 | [plan](anchor-runs/tuning-pass-1/ANC-032/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-032/development/result.json) |
| ANC-033 | 54/54 | 96/96 | 0/0 | [plan](anchor-runs/tuning-pass-1/ANC-033/planning/result.json) / [dev](anchor-runs/tuning-pass-1/ANC-033/development/result.json) |

Every selected bundle has exact semantic verdicts, identical repeated role tuples and zero regressions under `plan-anchor/question-tournament/2.0.0`. Rejected first rounds for ANC-022, ANC-026, ANC-029 development and ANC-030 development remain in the raw archives and append-only histories. No locked-statement amendment was requested by an evaluator; the three pending-definition anchors were not guessed.

## Production limits and provenance

Tournament acceptance uses the existing verdict projection at confidence threshold `0.0`, as required by the tuning protocol. Production remains at `0.6`; no threshold or role-to-verdict mapping changed. Raw low-confidence role answers are preserved in every independent audit. A passing semantic tournament does not resolve those confidence gaps, ratify anchor testability, or prove a real plan follows an anchor. **No production plan gate, development gate or coverage PASS was run or claimed.**

The existing `jev_judge.py` confidence projection has a separate observed limitation: a low-confidence positive `evidenced` answer can still return `satisfied`, and `low_questions_of` omits evidence. We reported this to the lead without changing imported tooling. Audit files distinguish the existing-code projection from raw low-role confidence so it cannot be mistaken for policy compliance.

Jev responses did not provide server run IDs. The reported run IDs are deterministic invocation/candidate/case/repeat correlation identifiers, bound to exact requests/raw responses and hashes; they are not invented server-issued IDs. Endpoint binding was verified as `https://api.typesafe.ai/v1/systemone`, with no endpoint override. ANC-024 includes an extra unchanged endpoint-confirmation run and a resolution note; its original results remain primary and both evidence sets are retained. Two evaluators disclosed reading a generic driver that also contained another anchor’s authoring; their own frozen oracles remained independently derived before their own Jev answers. Those disclosures remain in local bindings, and subsequent evaluators used the generic-only executor.

Raw archives may contain local absolute paths in bindings/requests. Canonical histories remain source-neutral and hashed; local paths are outside their authority/history identities. Credential scanning found no API key in exported JSON.

## Next steps

Define the approved exception path for ANC-018/020/021 before their tuning. Preserve locked-statement amendment rules if Rand changes authority. After confidence/testability prerequisites and the remaining three anchors are resolved, run the real anchor planning gate against the current complete sprint items; the existing plan-review bead must not close merely because these tournaments accepted. Local commit only; no push or PR.
