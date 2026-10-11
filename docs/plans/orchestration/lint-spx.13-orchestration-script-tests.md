---
sprint: lint-spx.13
bead: lint-spx.13
epic: lint-spx
status: complete
branch: chore/orchestration-script-tests
worktree: /Users/randlee/github/sc-lint-worktrees/chore/orchestration-script-tests
pr_target: chore/orchestration-beads-lifecycle
closure_type: contract
---

# lint-spx.13 — Tests for ported orchestration scripts; template/caller contract gaps

Layer 3 of the orchestration stack (on PR #167). Skill and agent files are
executable instructions: every script they call must work and be tested, and
every template must accept exactly what its caller is told to pass.

## Deliverables

1. Port the atm-core `origin/develop` tests for the scripts ported in layer 1
   and adapt them to sc-lint:
   - `.claude/skills/closing-triage/scripts/test_query_open_findings.py`
   - `.claude/skills/graph-orchestration/scripts/test_validate_findings.py`
   - `.claude/skills/triaging-findings/tests/test_check_dependencies.py`
   - `.just/tests/test_team_lead_roster_check.py` (place it where sc-lint's
     pytest gate discovers it)
   - new tests for `.claude/lib/sc_compose_dependency.py`
2. Wire every one of those tests into the gate that `just test` / `just lint`
   already runs (`sc_lint.run_pytests` or the Justfile pytest recipe). A test
   that exists but is not executed by the gate does not count. Show the gate
   output line that proves each file ran.
3. Corner cases, each with a test, for every script: missing input file,
   empty input, malformed JSON/TOML/Turtle, unknown finding id or phase,
   duplicate ids, missing optional dependency (`sc-compose`, `herdr`, `atm`
   not on PATH), non-zero exit codes and their messages, and for
   `roster_check.py`: alias present vs absent, Herdr agent unnamed (the state
   observed on 2026-09-19 after pane restarts), Herdr name not matching the
   alias, roster member missing from `.atm.toml`. Tests must not require live
   Herdr/ATM daemons: inject command output.
4. Template/caller contract gaps:
   - `ruthless-boundary-qa` agent exists but
     `codex-orchestration/ruthless-boundary-qa-assignment.json.j2` (+ sample
     vars) does not; port it, or remove the agent from every reviewer list.
     One or the other, consistently.
   - Two rust-best-practices assignment templates exist
     (`codex-orchestration/rust-best-practices-agent-assignment.json.j2` and
     `.claude/assets/sc-rust/quality-mgr/templates/rust-best-practices-assignment.json.j2`).
     Keep one; every caller names that one.
   - Add a test that composes **every** `.j2` under `.claude/skills/` and
     `.claude/assets/sc-rust/quality-mgr/templates/` with its committed sample
     vars, asserts success, and for `format: json` templates asserts the
     output parses and that its top-level keys equal the input keys the
     receiving agent's prompt declares in its fenced-JSON contract. Skip with
     an explicit reason (not silently) when `atm` is not on PATH.
5. EOF whitespace: `git diff --check develop..HEAD` is clean (currently flags
   `.claude/lib/__init__.py`, `.claude/lib/sc_compose_dependency.py`,
   `closing-triage/scripts/query_open_findings.py`).

## Acceptance criteria

- `just lint` and `just test` pass and visibly execute the new tests.
- Every script under `.claude/skills/**/scripts/` and `.claude/lib/` has a
  test file; each corner case in Deliverable 3 maps to a named test.
- The compose-all-templates test passes and fails when a sample var is
  removed (demonstrate once in the close report).
- No reviewer list names an agent without an assignment template, and no
  template is unreferenced.
- This doc's frontmatter is `status: complete`; bead `lint-spx.13` claimed and
  closed in tandem with the ATM task.

## Gate behavior

`just lint` and `just test` invoke `sc_lint.run_pytests`, which runs the
repository unittest suite and then the six ported pytest files explicitly.
Template composition uses `atm compose` when `atm` is on `PATH`; a CI image
without it instead performs a named Jinja2 syntax/`StrictUndefined` check with
autoescape disabled. The canonical triage-record test names and reports its
`sc-compose` Python-binding skip when that optional binding is unavailable.

## This sprint does not close

- plan-hardening templates (bead `lint-spx.12`).
- Any Rust source change.
