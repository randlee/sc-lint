# Team Messaging Protocol

This protocol is mandatory for all `sc-lint` ATM team communications.

## Required Flow

1. Read an actionable ATM assignment and immediately run `atm task start <task-id> "<one-line plan>"`.
2. Execute the requested work.
3. Send a concise progress report after pushing.
4. After validation passes, close the ATM task with `atm task close <task-id> completed --stdin`.
5. No silent processing; report blockers promptly.

Every assignment is dispatched as `atm task assign <agent> --task-id <task-id> --template <template.j2> --vars <vars.json>`. Do not hand-render an assignment and send it with `atm send`.

## Messaging Rules

- Use `atm send` for progress, blockers, and non-task information.
- Do not use `atm ack` unless the message explicitly requires acknowledgement.
- Template installation on the daemon host uses `~/.atm/templates/codex-orchestration/`; refresh installed copies after template changes.
