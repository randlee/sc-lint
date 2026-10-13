# Plan QA round 3: FAIL

`validate-plan --root lint-phase-k` exit 0: plan valid, 9 sprints (run from the plans/phase-k worktree; origin/integrate/phase-k at c79c210aad6ff8725f64385aea454361b1d6f053; k-5 deferred/scope-excluded). Step b clean.

- lint-k-8 important plan-scope-reviewer deliverable 1 (R1-line-095 / PLAN-SCOPE-018) OPEN: root description calls its allowed_unused_dependencies migration template 'identical to k-8 D3' but the strings differ (backticks, placeholder grammar `{package}.{dependency_key}` vs `<package>.<dependency-key>`, wording). Two non-identical guidance strings remain. Fix: make the root string byte-identical to k-8 D3, or replace the root block with a one-line pointer to k-8 D3 as the sole authoritative string (drop the 'identical' claim) and add the k-8 AC2 assertion that the emitted diagnostic equals that single string. Verified directly against live lint-phase-k and lint-k-8 descriptions.

Round 3 fix verification (filing reviewers only, 8 carried ids): 7 fixed, 1 open, 0 regressed.
- R1-line-084 fixed (k-9 sizing rationale, D1-D6 breakdown, AC1-AC3 mapped)
- R1-line-085 fixed (k-10 sizing rationale, finite inventory at 13fd141)
- R1-line-086 fixed (k-10 D5 staged branch-built binary + docs bundle; AC3 uses only STAGED_SC_LINT)
- R1-line-088 fixed (k-6, k-7, k-8, k-9 acceptance numbered and mapped)
- R1-line-090 fixed (k-9 D1 Rust type definitions, SystemAdapter additions, workflow.rs serializer owned)
- R1-line-095 OPEN (see Findings)
- R1-line-126 fixed (ceremony-qa: k-10 D3/D4 reference owning contracts)
- R1-line-128 fixed (ceremony-qa: k-3 single aggregation contract, boilerplate removed)

- plan-scope-reviewer (fix verification): 5 fixed, 1 open important, 0 regressed
- ceremony-qa (fix verification): 2 fixed, 0 open
- No other reviewer ran (fix-verification precedence; no req-qa/arch-qa sweep, no screening panel).

- Non-finding observations (plan-scope-reviewer): k-9 D1 `plan_consumer_lint` signature has no parameter for the lint_profiles name-to-members map; serialized spelling of SkipReason variants vs `unsupported_cross_command`/`unsupported_cross_target` strings is unstated. (ceremony-qa): k-3 description repeats the acceptance_criteria field verbatim; near-duplicate compatibility sentences in k-3 scope boundary; k-10 AC3 is a long paragraph.
- Next round: plan-scope-reviewer, R1-line-095 only.