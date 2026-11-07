# ADR-023 proposed amendment: embedded adoption plugin

**Status: Pending Rand approval.** This is proposed text; it does not change the accepted ADR-023 until Rand approves it and the approved amendment is applied to that ADR. Do not dispatch lint-p-3 before approval.

## Proposed decision

The single source of the sc-lint adoption plugin moves to `crates/sc-lint/assets/plugin/`. The existing `sc-lint-adoption` marketplace entry retains its identity and points to this root in `.claude-plugin/marketplace.json`. The root contains its own plugin manifest and the shipped `skills/` and `agents/` directories; plugin paths stay inside the plugin root.

`crates/sc-lint/build.rs` enumerates those directories at build time and emits a sorted, slash-normalized embedded-asset list used by `sc-lint init --just`. The only excluded subtree is `skills/sc-lint-adoption/evals/`. Init replaces each owned skill directory as a whole, so existing files in that directory, including an old `evals/` subtree, are not preserved. The owned skill and agent files are written by init from the binary, consistent with ADR-023's single-source ownership contract.

The existing frozen recipe contract includes signatures: in particular `lint selector='' *scope:` continues to forward selector and scope to bootstrap.

## Marketplace compatibility and migration

The marketplace and plugin names remain unchanged. Existing installations that use the marketplace identity must refresh or reinstall the plugin after the marketplace source is updated, so the plugin manager acquires the new root. A hard-coded reference to `packages/sc-lint-adoption/` is not a supported continuing install path after that directory is retired; the migration guides and changelog must name that break and the refresh/reinstall remedy. Verify the refresh/reinstall path with an existing old-path installation and a new installation before claiming compatibility; do not assume a cached plugin copy updates itself.

## Approval and implementation

Rand's approval of this amendment is a prerequisite for dispatching lint-p-3. On approval, apply this text to ADR-023 and record the approved status on the planning branch before dispatch. Until then, the existing accepted ADR-023 text remains authoritative.
