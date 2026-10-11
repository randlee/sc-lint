---
sprint: lint-spx.1
bead: lint-spx.1
epic: lint-spx
status: complete
branch: chore/codex-orchestration-task-assign
worktree: /Users/randlee/github/sc-lint-worktrees/chore/codex-orchestration-task-assign
pr_target: develop
closure_type: contract
---

# lint-spx.1 — Port atm-core codex-orchestration to sc-lint (beads + `atm task assign`)

## Goal

sc-lint orchestration dispatches every assignment as

```bash
atm task assign <agent> --task-id <bead-id> --template <template> --vars <vars.json>
```

where `<bead-id>` is the `bd` issue id of the work item and is also the
`task_id` key in the vars file. Rendering a template by hand and sending the
text with `atm send` is retired.

## Source of truth

Start from atm-core `origin/develop` at `5b284f057` (the local atm-core
checkout is behind; read with
`git -C /Users/randlee/github/atm-core show origin/develop:<path>`), not from
the current sc-lint files:

- `.claude/skills/codex-orchestration/` (SKILL.md and every `.j2`)
- `.claude/agents/quality-mgr.md`, `.claude/agents/qa-triage.md`
- `.claude/skills/triaging-findings/`, `.claude/skills/quality-management-gh/`
- `.claude/skills/team-lead/`, `.claude/skills/phase-orchestration/`
- `docs/team-protocol.md` "Required Flow" (task start / work / task close)

## Deliverables

1. Replace sc-lint `.claude/skills/codex-orchestration/` with the atm-core
   version, adapted to sc-lint (team `sc-lint`; developers `clint` = harder
   tier, `cfast` = fast tier for easy changes; lead default `team-lead`, any
   identity such as `flint` may hold the role via the `lead` variable).
2. Beads integration, stated once in SKILL.md and reflected in templates:
   - every dev, fix, review, and QA assignment is backed by a bead created
     before dispatch; bead id == ATM `task_id`;
   - release work hangs under a release epic (`bd create --parent`);
   - after `/triaging-findings`, each promoted finding becomes a child bead of
     the epic and that bead id is the fix task id;
   - assignee closes the ATM task; the lead closes the bead after accepting
     the result (developers do not close beads on push).
3. Port the dependent prompts listed under "Source of truth" so the
   start/close lifecycle, QA-1 vs QA-2+ reviewer sets, and tiered fix routing
   are consistent across skill, agents, and `docs/team-protocol.md`.
4. Reconcile every reference that does not exist in sc-lint. For each, either
   port the file or remove/replace the reference, and list the decision in the
   close report: `ruthless-boundary-qa` agent + assignment template,
   `docs/development/gh-stack-guidelines.md`, `just validate`, post-mortem
   references, `sprint-plan.md.j2`, `recommended_agent` roster. The stack rules
   in `CLAUDE.md` (stacks rooted on `develop`, merge forward, never
   `gh stack sync`/`rebase`) win over atm-core's `integrate/phase-N` wording.
5. Template install step documented for the daemon host
   (`~/.atm/templates/codex-orchestration/`), and the installed copies
   refreshed.
6. `CLAUDE.md` and `AGENTS.md` updated and mirrored where they describe
   orchestration or team members; `docs/project-plan.md` gains this sprint
   entry.

## Acceptance criteria

- `atm compose --template <each template> --vars <sample>` succeeds for every
  template; sample vars files are committed next to the templates or under
  the skill's `vars/` directory, using a bead id as `task_id`.
- `grep` finds no reference in the ported files to an agent, doc, script, or
  `just` recipe that is absent from sc-lint.
- No template takes an assignee variable or names its recipient.
- `just lint` passes (docs/prompt-only change; no Rust changes expected).
- This doc's frontmatter is set to `status: complete` at closeout.

## This sprint does not close

- Re-dispatching the in-flight stack #165 QA round.
- Any Rust source change.
