# ADR-023 proposed amendment: embedded adoption plugin

**Status: Pending Rand approval (gate bead `lint-bg1s`, which blocks lint-p-3).** This is proposed text; it does not change the accepted ADR-023 or ADR-020 until Rand approves it and it is applied to those ADRs on the planning branch. Until then the accepted text remains authoritative.

## Proposed decision

The single source of the sc-lint adoption plugin moves to `crates/sc-lint/assets/plugin/`. The existing `sc-lint-adoption` marketplace entry retains its identity and points to this root in `.claude-plugin/marketplace.json`. The root contains its own plugin manifest and the shipped `skills/` and `agents/` directories; plugin paths stay inside the plugin root.

`crates/sc-lint/build.rs` embeds every file under those directories for `sc-lint init --just`. The only excluded subtree is `skills/sc-lint-adoption/evals/`. Init deletes each owned skill directory and writes it again, so existing files in that directory, including an old `evals/` subtree, are not preserved. The owned skill and agent files are written by init from the binary, consistent with ADR-023's single-source ownership contract.

The existing frozen recipe contract includes signatures: in particular `lint selector='' *scope:` continues to forward selector and scope to bootstrap. The interpreter variable `_sc_lint_python` in `.sc-lint/justfile` is part of that frozen contract: it is the only interpreter selection in the just layer, and the consumer starter `Justfile`'s `test` recipe uses it.

## Marketplace compatibility and migration

The marketplace and plugin names remain unchanged. Existing installations that use the marketplace identity must refresh or reinstall the plugin after the marketplace source is updated, so the plugin manager acquires the new root. A hard-coded reference to `packages/sc-lint-adoption/` is not a supported continuing install path after that directory is retired; the migration guides and changelog name that break and the refresh/reinstall remedy.

## ADR-023 §5: one source per sc-lint-owned file

Add to §5:

> This repository is itself a consumer: its tracked `.sc-lint/bootstrap.py` is the output of `sc-lint init --just`, not a second source, and `bindings/sc-lint-py/python/sc_lint/tests/test_consumer_contract.py` keeps it byte-equal to `crates/sc-lint/assets/bootstrap.py`. This repository's vendored `.github/actions/setup-sc-lint/` and `.github/scripts/setup_sc_lint_source.py` (with its test) are deleted in phase-p (lint-p-4); consumers' copies of that action are sc-publish-owned and are removed per their migration guide (§3).

## ADR-023 §6: consumer canary

Replace the consumer list with:

> Release installs the release candidate and runs `just setup`, `just lint` and `just test` in atm-core and sc-compose (ANC-010), and generates one project from sc-runtime scaffolding with `just new` (which runs `sc-lint init --just`, `just setup`, `just lint`, `just test`). Any failure blocks the release. (Rand 2026-11-07: "I expect 90% of new sc-lint consumers will be based on sc-runtime scaffolding.")

Add `docs/migrations/0.8.7/sc-runtime.md` and `docs/migrations/0.8.7/wyvern.md` to the guide list after "Each consumer has its own guide under `docs/migrations/0.8.7/`".

## ADR-020: retire the adoption golden

Append to the l-5 clarification 1:

> Retired in phase-p (0.8.7): `tests/adoption/golden/` recorded the output of the kit's `install.py`, which ADR-023 §5 deletes; the baseline is removed with its producer. The bootstrap behaviour tests move to `tests/bootstrap/`.
