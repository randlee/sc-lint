# Minimal Marketplace Constraints

This document records the repo-local planning constraints for advertising the
`sc-lint-version` adoption skill through a minimal repo-local Claude Code
marketplace.

## Source-Repo Publication Surfaces

The planned source-repo publication set contains both:

- `.claude-plugin/marketplace.json`
- `packages/sc-lint-version-adoption/.claude-plugin/plugin.json`

## Marketplace Shape

- the source-repo marketplace entry advertises the
  `sc-lint-version-adoption` package from the local `packages/` tree
- the package-level plugin manifest provides the package identity and version
- marketplace publication remains separate from skill design

### Required Fields

`/.claude-plugin/marketplace.json` must contain:

- `name`
- `plugins[].name`
- `plugins[].source`

`/packages/sc-lint-version-adoption/.claude-plugin/plugin.json` must contain:

- `name`
- `version`
- `description`
- `author`

## Separation Rules

- `C.4` closes skill design and consumer-adoption guidance
- `C.5` closes marketplace publication only
- no sprint in this line should mix skill-design acceptance with marketplace
  publication acceptance
