# Agent JSON configure fixtures

These roots are sanitized, minimal derivations of the F.3a wizard handoff
contexts. They contain only the conventional path-presence facts that F.2 is
allowed to observe. They are not consumer snapshots and contain no copied
Justfile, workflow, source, credential, or shell content.

The request authority remains the accepted F.3a page-to-JSON mapping in
[`docs/sc-lint/configure-wizard-fixtures/`](../../../../docs/sc-lint/configure-wizard-fixtures/).
`test_agent_json_and_skill.py` proves these roots collect to those documented
contexts and submits their documented requests through the public JSON CLI.
