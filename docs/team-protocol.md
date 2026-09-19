# Team Messaging Protocol

This protocol is mandatory for all `sc-lint` ATM team communications.

## Required Flow

1. Read an actionable ATM assignment and immediately run `atm task start <bead-id> "<one-line plan>"`.
2. Execute the assigned work. The Beads issue id is the ATM task id.
3. Send a concise progress report after pushing; this does not close either task.
4. After validation passes, close the ATM task with `atm task close <bead-id> completed --stdin` and include the completion report.
5. The lead reviews the result and closes the backing Beads issue after acceptance. Developers do not close beads when pushing.

Every assignment is dispatched as:

```sh
atm task assign <agent> --task-id <bead-id> --template <template.j2> --vars <vars.json>
```

The vars file must contain the same value under `task_id`. Do not hand-render an assignment and send it with `atm send`.

## Messaging Rules

- Read every task message. Start only the task that is ready; queued work waits for the current task close.
- Use `atm send` for progress, blockers, and non-task information. Do not use `atm ack` unless the message explicitly requires an acknowledgement.
- If blocked, report the blocker promptly. If the task cannot be completed, close it with `refused` and the reason.
- Use the branch, commit, and validation result in progress and close reports when relevant.
- Template installation on the daemon host uses `~/.atm/templates/codex-orchestration/`; refresh installed copies after template changes.

## Example

```sh
atm task start lint-123 "Read the sprint document and inspect the current branch."
atm task assign clint --task-id lint-123 --template ~/.atm/templates/codex-orchestration/dev-template.xml.j2 --vars /tmp/lint-123.json
atm task close lint-123 completed --stdin
```
