# Team Messaging Protocol

This protocol is mandatory for all `sc-lint` team communications.

## Required Flow

1. Immediately acknowledge every team message received.
2. Execute the requested task.
3. Send a completion message with a concise summary of what was done.
4. Receiver immediately acknowledges completion.
5. No silent processing.

## Good Patterns

- Request received:
  - `ack, working on PR #12 review updates now.`
- Completion sent:
  - `task complete: updated boundary docs, reran lint, pushed abc1234.`
- Completion acknowledged:
  - `received. QA pass starting now.`

## Notes

- If blocked, send an immediate acknowledgement plus blocker status.
- If work will take time, send periodic progress updates.
- Prefer concise, explicit messages with branch, commit, and validation context.
