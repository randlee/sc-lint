# Homebrew Tap Templates (not installable)

The formulas in `Formula/` are bootstrap templates kept in this repository. They
carry the placeholder version `0.0.0` and zero SHA256 values, so
`brew install` from this directory fails by design.

Install sc-lint from the published tap instead:

```bash
brew install randlee/tap/sc-lint
```

The published tap is `randlee/homebrew-tap`. For each release,
`.github/workflows/homebrew-publish.yml` renders `Formula/sc-lint.rb` from
`release/homebrew/formula.rb.j2` with the release's archive URLs and SHA256
values, and commits the result to that tap. The rendered formula includes the
linux ARM64 archive and installs `share/sc-lint/sc-lint-docs`.

Do not hand-edit these templates to ship a release. Change
`release/homebrew/formula.rb.j2` or `release/publish-artifacts.toml`, then let
the publish workflow render the tap.
