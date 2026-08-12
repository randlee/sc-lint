# Sprint E.6 Closure Checklist

This worktree-local checklist is the implementation and closure record for
Sprint E.6. It supplements, but does not replace,
[the sprint plan](./sprint-E6.md). Items are completed only after their listed
evidence is checked on this branch.

## Delivery checklist

- [x] **Baseline and scope.** Confirm this branch contains the current
  `integrate/phase-E` head and E.1/E.3/E.5 contracts; keep repository
  dogfooding and end-to-end consumer acceptance out of scope for E.7.
  **Closure evidence:** `HEAD` and `origin/integrate/phase-E` were both
  `9a6cbf0` before implementation; the E.1/E.3/E.5 requirement surfaces were
  checked and E.7-only dogfooding remains excluded.

- [x] **Action contract and provenance requirements.** Create
  `docs/sc-lint/github-action-requirements.md`, update `REQ-PRODUCT-022`, and
  reconcile the product architecture. Specify release URL/checksum provenance,
  platform mapping, compatibility preflight, supported inputs (`setup`,
  `lint`, `test`), outputs, major/exact pinning, cache/offline policy, and
  stable recovery codes. **Closure evidence:** every requirement has an
  implementation, metadata, fixture, example, or documentation reference.
  **Completed evidence:** `GA-001` through `GA-008` define all required
  contract areas and each names its concrete validation surface; the product
  requirement and architecture now link to the Action record.

- [x] **Versioned action metadata and runtime.** Add the root reusable Action
  metadata plus a platform-neutral runtime that downloads only the selected
  release archive and its checksum manifest, verifies SHA-256 before
  extraction, locates the shipped binary and `sc-lint-docs`, exposes their
  paths, runs the E.1 compatibility preflight, and invokes the selected
  consumer operation. **Closure evidence:** metadata schema passes and static
  inspection proves no Cargo, source-checkout, or analyzer-package fallback.
  **Completed evidence:** `node --check action/index.js` passes; `action.yml`
  declares Node 20, the supported input/output surface, and the root Action
  entry point; the runtime contains only release download, checksum, archive,
  compatibility, docs, and consumer-command paths.

- [x] **Stable failure and output contract.** Make unavailable artifact,
  checksum mismatch, incompatible configured minimum, and invoked-command
  failure distinct Action errors with recovery instructions. **Closure
  evidence:** focused tests assert each stable code, recovery text, and output
  shape; success exposes binary/docs/version outputs. **Completed evidence:**
  the Node fixture suite asserts all four error-code/recovery categories and
  verifies the three outputs plus `GITHUB_PATH` on success.

- [x] **Published-layout action fixtures.** Add deterministic local release
  archive/checksum fixtures and exercise `setup`, `lint`, and `test` for Linux,
  macOS, and Windows platform mappings without a network request. **Closure
  evidence:** fixture runner passes all nine operation/platform cases,
  verifies offline docs discovery, and proves no disallowed fallback.
  **Completed evidence:** `node --test action/test/action.test.cjs` passes all
  nine Linux/macOS/Windows operation cases against generated tar.gz/zip
  published layouts, plus checksum, unavailable-artifact, compatibility,
  command-failure, and fallback regressions.

- [x] **Release and CI validation wiring.** Add the Action major-version
  publication/tag wiring and Action fixture validation matrix without adopting
  the Action for this repository's own consumer CI. **Closure evidence:**
  workflow syntax is valid, matrix declares Linux/macOS/Windows, and release
  wiring updates the documented major tag only after release publication.
  **Completed evidence:** `action-fixtures.yml` defines the three hosted
  runner lanes, and `publish-action-major-tag` depends on the successful
  release job before force-updating only the documented `v1` Action tag.

- [x] **Installed documentation and consumer example.** Update
  `docs-bundle/ci.md` with a complete Action workflow using stable-major and
  exact pins, least-privilege permissions, cache/offline behavior, outputs,
  and recovery. Update troubleshooting with Action-specific stable codes.
  **Closure evidence:** examples contain no Cargo/package-manager/copied
  scripts and docs remain discoverable from the staged local bundle.
  **Completed evidence:** the CI guide contains stable-major and exact-SHA
  examples with least-privilege permissions, mirror/cache/offline/output
  policy, and the troubleshooting table contains every Action code.

- [x] **Traceability, review, and final validation.** Cross-check the action
  requirements against metadata, fixtures, workflow examples, and docs; run
  the required Rust gate plus Action validation. **Closure evidence:** no
  untraced requirement, clean status, `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` all pass. **Completed evidence:** GA-001 through
  GA-008 each resolve to an Action, fixture, release-workflow, or bundle-doc
  surface; `just test` includes the Action test suite so the repository's
  aggregate test command covers both Rust and Action contracts.

## Closure-review pass 1

The completed implementation was compared against every E.6 deliverable and
acceptance criterion. The root Action uses only E.5 archive/checksum inputs,
preflights before consumer operations, supplies stable failures and outputs,
and has generated published-layout fixtures covering setup/lint/test on the
three required platform families. Requirements, architecture, installed CI
guide, troubleshooting, release tag wiring, and CI validation matrix are all
present. E.7-only repository dogfooding and external end-to-end acceptance were
not added.

## Closure-review pass 2

The second review identified two closure risks and resolved both before final
validation: GitHub release downloads must follow the release asset redirect,
and `version` must be validated as SemVer before becoming a release path.
Regression coverage now exercises the latter; the published-layout fixtures
continue to prove the platform-specific archive contract. No unclosed E.6 item
remains.
