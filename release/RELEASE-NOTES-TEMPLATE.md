# sc-lint Release Notes

## Release

- Version:
- Date:
- Release owner:
- Approval:

## Summary

Briefly describe what changed in this release.

## Included Crates

Do not hand-maintain this list. `release/publish-artifacts.toml` is the source of
truth for the publish set; generate the list from it so it cannot drift:

```bash
python3 scripts/release_artifacts.py list-artifacts \
  --manifest release/publish-artifacts.toml \
  --publishable-only | sed 's/^/- `/;s/$/`/'
```

Paste the output below, replacing this block. The order it prints is the manifest
publish order, which is the order the release workflow publishes in.

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
