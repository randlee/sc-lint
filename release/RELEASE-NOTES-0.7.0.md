# sc-lint Release Notes

## Release

- Version: 0.7.0
- Date: 2026-10-11
- Release owner: clint (release preparation; publication is handled separately)
- Approval: Rand authorized the version bump and changelog promotion.

## Summary

0.7.0 delivers the Phase I analyzer-configuration and CLI improvements, clarifies
consumer-owned Justfile behavior, and restores additive portability prefix matching.

## Included Crates

<!-- BEGIN INCLUDED-CRATES -->
- `sc-lint-directives`
- `sc-lint-schema`
- `sc-lint-analyzer-support`
- `sc-lint-attributes`
- `sc-lint-boundary`
- `sc-lint-portability`
- `sc-lint-runtime`
- `sc-lint`
<!-- END INCLUDED-CRATES -->

## Major Changes

- Analyzer-specific configuration tables support common enablement, rule, and path
  controls; portability additionally supports configured home environments and
  additive Unix path prefixes.
- The boundary, portability, and runtime lint commands accept `--rule-filter` and
  return exit code 10 when the selected lint verdict fails.
- Unknown analyzer-table keys fail configuration validation with exit code 3.

## Migration Notes

- Existing Justfiles remain consumer-owned. The adoption kit adds its recommended
  `just test` recipe only when the consumer does not already provide one.
- `unix_path_prefixes` extends, rather than replaces, the built-in PORT-001 and
  PORT-006 prefix detection list.

## Validation

- `just lint` — PASS
- `just test` — PASS

## Packaging / Publication Notes

- This preparation changes no tags, GitHub releases, crate publications, or wheel
  publications.
- Version 0.6.0 was cut to `main` but remains unpublished: it has no Git tag or
  GitHub Release, crates.io currently tops out at 0.5.0, and PyPI has no
  `sc-lint` distribution. The publisher owns the release-channel audit.

## Follow-Up Items

- The publisher will audit release channels and publish only through the approved
  release process.
