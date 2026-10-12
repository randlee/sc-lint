# Plan Anchor plugin

`plugins/sc-plan-anchor/` is a byte-for-byte copy of the `sc-plan-anchor`
package from [randlee/plan-anchor](https://github.com/randlee/plan-anchor) at
main commit `1184682e98b4e48f54f6f5dce796c2834a86b74f` (skill `plan-anchoring`
0.4.5). Do not edit it here; update it by copying the package again at a new
pin.

The package carries one shared skill (`skills/plan-anchoring/`) and a separate
set of agent definitions for each harness:

| Harness | Manifest | Agents | Models |
| --- | --- | --- | --- |
| Claude Code | `.claude-plugin/plugin.json` | `.claude/agents/` | Sonnet (gates), Opus (tuning) |
| Codex | `.codex-plugin/plugin.json` | `.codex/agents/` | Luna (gates), Sol (tuning) |

Both harnesses install the same package through this repository's own
marketplace.

## Claude Code

The entry is in `.claude-plugin/marketplace.json`:

```text
/plugin marketplace add ./
/plugin install sc-plan-anchor@sc-lint-marketplace
```

Invoke the skill as `/sc-plan-anchor:plan-anchoring`.

## Codex

The entry is in `.agents/plugins/marketplace.json`:

```bash
codex plugin marketplace add .
codex plugin add sc-plan-anchor@sc-lint-marketplace
```

Or open `/plugins` and install **Plan Anchor**.

## Requirements

`python3` 3.9+, `bd` 1.x, and for live Jev operations `TYPESAFE_API_KEY` plus
`PLAN_ANCHOR_ALLOW_REMOTE_JUDGE=1`. See the upstream README for details.

## Validation

```bash
just plan-anchor-check
```

This resolves `anchor-plan-review`, `anchor-sprint-review` and `anchor-tuning`
for both `--harness claude` and `--harness codex`, and runs
`claude plugin validate plugins/sc-plan-anchor`. Codex has no offline
validate command; check its install in a throwaway home:

```bash
CODEX_HOME="$(mktemp -d)" sh -c 'codex plugin marketplace add . && codex plugin add sc-plan-anchor@sc-lint-marketplace'
```
