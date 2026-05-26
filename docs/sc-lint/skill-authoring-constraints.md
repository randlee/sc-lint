# Skill Authoring Constraints

This document records the repo-local planning constraints for the
`sc-lint-version` consumer-adoption skill in Phase `C`.

## Skill Shape

- the adoption guidance is packaged as one repo-local Claude Code skill
- the planned skill surface is:
  - `.claude/skills/sc-lint-version-adoption/SKILL.md`
- the skill uses explicit versioned frontmatter
- the skill description stays narrow to `sc-lint-version` adoption and usage

## Consumer Guidance Coverage

- the skill points to one authoritative adoption document:
  - `docs/sc-lint/version-adoption.md`
- the adoption workflow explains what a consuming repository must provide for:
  - Rust public API baselines
  - CLI interface checks
  - RPC/socket interface checks when present
- the guidance explicitly defines:
  - repo-side harness responsibilities
  - fixture responsibilities
  - simulator/transcript responsibilities when present
  - normalization hooks only when unstable values cannot be removed at the
    canonical artifact layer

## Reuse Expectations

- consuming repositories should reuse existing CLI testability surfaces where
  available
- consuming repositories should reuse existing simulators or transcript
  infrastructure where available
- the skill should reduce bespoke adapter code rather than encourage every
  consumer to invent a different interface-exerciser flow
