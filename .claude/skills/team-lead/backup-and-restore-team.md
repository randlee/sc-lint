---
name: backup-and-restore-team
version: 0.2.0
description: Procedure for backing up and restoring the sc-lint team. Referenced by the team-lead skill when a session ID mismatch is detected.
---
# Team Backup And Restore Procedure

Follow this procedure when Step 1 of the `team-lead` skill detects a session id
mismatch and a full team restore is required. This is the startup or `clear`
path where the live `SESSION_ID` changed and no longer matches
`leadSessionId`.

Do not use this procedure for same-session compaction or resume when the
session id still matches. Use `/restore-team-communications` for that lighter
repair path.

## Step 2 — Backup Current State

Always back up before modifying the team:

```bash
atm teams backup sc-lint
```

Also back up the Claude Code project task list separately:

```bash
BACKUP_PATH=$(ls -td ~/.claude/teams/.backups/sc-lint/*/ | head -1)
cp -r ~/.claude/tasks/sc-lint/ "$BACKUP_PATH/tasks-cc"
echo "CC task list backed up to $BACKUP_PATH/tasks-cc"
```

Note: the explicit `tasks-cc/` copy preserves the local task bucket contents
and highwatermark alongside the standard ATM team backup.

## Step 3 — Clear Stale Team State

```text
TeamDelete
```

Then remove the stale team directory so the next create uses the correct name:

```bash
rm -rf ~/.claude/teams/sc-lint
```

If `TeamDelete` already removed the directory, the `rm -rf` is harmless.

## Step 4 — Create Team

```text
TeamCreate(team_name="sc-lint", description="sc-lint development team", agent_type="team-lead")
```

Verify that the returned team name is exactly `sc-lint`. If it is not, stop.

Note: `/restore-team-communications` reuses this same `TeamCreate` primitive
when communications are broken after compaction or resume, but that path should
prove the failure first and avoid this destructive backup/delete/restore flow
unless the lighter repair fails.

## Step 5 — Restore Team Members And Inboxes

```bash
atm teams restore sc-lint --from ~/.claude/teams/.backups/sc-lint/<timestamp>
```

Verify members:

```bash
atm members
```

### Reconcile Against `.atm.toml` And Confirm Herdr Backend

sc-lint runs under herdr, not tmux. `.atm.toml` is launch-only — it is not the
source of truth for the ATM roster, so treat any drift between it and the
roster as a roster bug to fix, not an `.atm.toml` edit.

```bash
atm members --json
```

For each member, confirm `"backend": "herdr"` and a `"herdrSession"` value
(currently `default`). A member showing `"backend": "tmux"` or a
`tmux_pane_id` is stale and its nudges will silently fail — fix it:

```bash
atm teams update-member sc-lint <name> --backend herdr --session default
```

Then compare the member names against every `[[rmux.windows.panes]]` entry in
`.atm.toml` (excluding any pane meant to stay a free/manual terminal, e.g.
`spare`). Add any pane declared in `.atm.toml` but missing from the roster:

```bash
atm teams add-member sc-lint <name> --agent-type <type> --model <model> \
  --backend herdr --session default --home-dir "$(pwd)"
```

Finally, confirm each herdr agent is addressable by its roster name — ATM's
herdr backend resolves the send target by agent name, not by pane label:

```bash
herdr pane list        # find pane_id for each label
herdr agent get <name> # should NOT return agent_not_found
herdr agent rename <pane_id> <name>   # if it does, rename to match
```

Do not add `[[atm.post_send_hooks]]` entries or tmux pane-id fields to
`.atm.toml` to work around this — the fix belongs in the ATM roster and the
herdr agent names, never in `.atm.toml`.

If unexpected ghost members exist, trim the config manually:

```bash
python3 -c "
import json
path = '/Users/randlee/.claude/teams/sc-lint/config.json'
with open(path) as f:
    cfg = json.load(f)
keep = ['team-lead', 'clint', 'quality-mgr']
cfg['members'] = [m for m in cfg['members'] if m['name'] in keep]
with open(path, 'w') as f:
    json.dump(cfg, f, indent=2)
print('Members:', [m['name'] for m in cfg['members']])
"
```

Adjust the `keep` list if additional named teammates are intentionally active.

## Step 6 — Restore Claude Code Task List

```bash
BACKUP_PATH=$(ls -td ~/.claude/teams/.backups/sc-lint/*/ | head -1)
if [ -d "$BACKUP_PATH/tasks-cc" ]; then
  mkdir -p ~/.claude/tasks/sc-lint
  cp "$BACKUP_PATH/tasks-cc/"*.json ~/.claude/tasks/sc-lint/ 2>/dev/null || true
  MAX_ID=$(ls ~/.claude/tasks/sc-lint/*.json 2>/dev/null \
    | xargs -I{} basename {} .json \
    | sort -n | tail -1)
  [ -n "$MAX_ID" ] && echo -n "$MAX_ID" > ~/.claude/tasks/sc-lint/.highwatermark
  echo "Task list restored. Highwatermark: $MAX_ID"
else
  echo "No tasks-cc/ in backup — task list not restored."
fi
```

The Claude Code UI task panel may not show restored tasks until one task is
created through the task tool.

## Step 7 — Verify Team Health

```bash
atm members
atm inbox
gh pr list
```

Communication verification is also mandatory:
1. `SendMessage` to another Claude teammate to prove Claude-side routing works.
2. `atm send` to a non-Claude model to prove ATM mailbox routing works.
3. `atm send` to Codex and confirm the Codex-side nudge fires.

## Step 8 — Read Project Context

1. Read `docs/project-plan.md`.
2. Recreate pending tasks if the task list is empty.
3. Output a concise project summary:
   - current phase and status
   - open PRs
   - active teammates and their last known task
   - next sprint or sprints ready to execute

## Step 9 — Notify Teammates

```bash
atm send clint "New session (session-id: <SESSION_ID>). Team sc-lint restored. Please acknowledge and confirm status."
```

If no response arrives within about 60 seconds, nudge via herdr — sc-lint has
no backing tmux session, so `tmux send-keys` is not a valid fallback here.
Preferred structured nudge payload when task metadata is available:

```text
<atm><action>read atm</action><action>ack <TASK-ID></action><action>execute assigned task</action><when idle="immediate" busy="after-current-task"/><console announce="concise" pause="false"/></atm>
```

Fallback plain-text nudge via herdr:

```bash
herdr pane list                              # find pane_id for the target agent's label
herdr agent prompt <name> "read atm for task <TASK-ID> and complete it before stopping"
```

`herdr agent prompt` takes the roster member name directly once that agent has
been renamed to match (see Step 5); it fails with `agent_blocked` if the
target is mid-prompt-approval, and with `agent_not_found` if the herdr agent
was never renamed to the roster name.

## Common Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| `TeamCreate` returns random name | `~/.claude/teams/sc-lint` still exists | remove the directory and retry |
| `TeamDelete` says no team name found | fresh session with no active team context | expected, proceed |
| task list looks empty after restore | highwatermark mismatch or UI stale state | set `.highwatermark`, then create one real task |
| `atm send` fails with agent not found | member missing after restore | add the member back to the team |
| `atm send` returns `ATM_HERDR_AGENT_NOT_VISIBLE` warning | member's `backend` is `tmux`/stale, or the herdr agent was never renamed to the roster name | `atm teams update-member` to `--backend herdr`, then `herdr agent rename <pane_id> <name>` |
| roster missing a pane declared in `.atm.toml` (e.g. `publisher`) | `.atm.toml` edited without a matching `atm teams add-member` | `atm teams add-member sc-lint <name> --backend herdr --session default` |
| self-send or wrong identity routing | teammate launched with wrong `ATM_IDENTITY` | relaunch with the correct identity |
