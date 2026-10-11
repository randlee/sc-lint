# sc-lint Release Notes

## Release

- Version:
- Date:
- Release owner:
- Approval:

## Summary

Briefly describe what changed in this release.

## Included Crates

Do not hand-maintain this list. `release/publish-artifacts.toml` is rendered
from the sc-publish consumer input and is the source for the release package
set and publish order. Generate the list from it so it cannot drift:

```bash
python3 .github/scripts/release_artifacts.py list-publish-plan \
  --manifest release/publish-artifacts.toml |
  cut -d '|' -f 1 | sed 's/^/- `/;s/$/`/'
```

Paste the output below, replacing this block. The order it prints is the
publish order used by sc-publish.

<!-- BEGIN INCLUDED-CRATES -->
<!-- END INCLUDED-CRATES -->

## Major Changes

- 

## Migration Notes

- 

## Validation

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `just lint`

## Packaging / Publication Notes

- 

## Follow-Up Items

- 
