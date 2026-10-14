# lint-k-10.group-dev.4: D5 changed-example verification evidence

Remedy for sanity finding D5-F1 (lint-k-10.group-sanity). Verified at the
final K-10 documentation head `f176a63`: the K-10 layer `f32fab0` plus D1
`67cb07b`, D2 `ceee9c3`, D3 `dd6c873` and the policy-location reconciliation
`f176a63`. The run was on macOS (Unix); the Windows `sc-lint.exe` variant was
not run. No K-1..K-9 issue regression, bootstrap platform suite or unrelated
product test was run as docs evidence. Temporary paths are shown as `<tmp>`.

## Changed examples (AC2 scope)

These are the fenced blocks in `git diff 593641cd f176a63` whose lines the K-10
work added or changed. No JSON snippet changed.

| Id | Location at f176a63 | Lang | Kind |
| --- | --- | --- | --- |
| E1 | `docs-bundle/configuration.md:5-16` | toml | complete consumer config |
| E2 | `docs-bundle/configuration.md:66-79` | toml | fragment: analyzer tables |
| E3 | `docs-bundle/installation.md:31-37` | sh | complete install sequence |
| E4 | `docs-bundle/packages/sc-lint-boundary.md:106-123` | toml | fragment: boundary policy tables |
| E5 | `docs-bundle/packages/sc-lint.md:23-30` | sh | complete command sequence |
| E6 | `docs-bundle/troubleshooting.md:45-48` | toml | complete .just/lint-config.toml |
| E7 | `docs-bundle/using-sc-lint.md:43-46` | sh | command pair (subset of E5) |

E2 and E4 are fragments. They are labelled inside the one assembled
`sc-lint.toml` below, which is executed as the full example together with E1.
Only one assembly line does not come from a changed block: it names the `ci`
profile that E5/E7 select (`using-sc-lint.md:48-50`).

## Build and staging (AC3 a, b)

```text
$ cargo build --locked -p sc-lint --bin sc-lint        # rc=0
$ cp target/debug/sc-lint <tmp>/k10-stage/bin/sc-lint
$ cp -R docs-bundle       <tmp>/k10-stage/bin/docs-bundle   # diff -r == f176a63 docs-bundle
sha256(target/debug/sc-lint) == sha256(<stage>/bin/sc-lint) == 14cd95fb2f9b098cf852f341b5f09a26e590d008fb61396ba70c67cd8b7b763d
STAGED_SC_LINT=<tmp>/k10-stage/bin/sc-lint             # Unix value <stage>/bin/sc-lint
```

The analyzer companions (`sc-lint-boundary`, `sc-lint-portability` and
`sc-lint-runtime`, built with `cargo build --locked -p ... --bins`) were staged
beside the launcher, as `installation.md` requires, for the analyzer runs.

## AC3 commands

```text
$ <tmp>/k10-stage/bin/sc-lint docs
Documentation root: <tmp>/k10-stage/bin/docs-bundle

# sc-lint operator manual

This is the offline, version-matched documentation bundle shipped with the
`sc-lint` release. It is usable without a source checkout or network access.
Start with [installation](./installation.md), then follow the
[Just setup guide](./just-setup.md) for the recommended consumer workflow.

## Operator guides

- [Installation](./installation.md) — Homebrew, release archives, and CI.
- [Using sc-lint](./using-sc-lint.md) — setup, lint, test, and daily commands.
- [Configuration]([...       37 lines, sha256 9ae46eb69b8659bc] rc=0

$ <tmp>/k10-stage/bin/sc-lint docs --path
Documentation root: <tmp>/k10-stage/bin/docs-bundle
[...        1 lines, sha256 c628f6f4d38d57fe] rc=0

$ <tmp>/k10-stage/bin/sc-lint docs configuration
Documentation root: <tmp>/k10-stage/bin/docs-bundle/configuration.md

# Configuration

Consumer configuration lives in `sc-lint.toml`:

```toml
[tool.sc-lint]
minimum_version = "0.7.3"

[[tool.sc-lint.lint]]
name = "rust"
command = ["cargo", "check", "--workspace"]

[[tool.sc-lint.test]]
name = "tests"
command = ["cargo", "test", "--workspace"]
```

## Minimum version

`minimum_version` is parsed as SemVer and compared semantically. The installed
version must be greater than or equal to the floor; lexical string comparison
is not used. Missing, malformed, or [...      106 lines, sha256 cafb5206039142ae] rc=0

$ <tmp>/k10-stage/bin/sc-lint docs setup
Documentation root: <tmp>/k10-stage/bin/docs-bundle/installation.md

# Installation

`sc-lint` is the installed product. Consumers do not need to select a Cargo
package or copy analyzer scripts.

## Homebrew

The supported macOS/Linux Homebrew path is the `randlee/tap/sc-lint` formula:

```sh
brew tap randlee/tap
brew install randlee/tap/sc-lint
sc-lint --version
sc-lint docs
```

The formula installs executables in `bin` and this bundle as
`<prefix>/share/sc-lint/sc-lint-docs` (the formula-owned `pkgshare`);
documentation remains local and offline.

## Relea[...       67 lines, sha256 6d25bff93330041c] rc=0

$ python3 scripts/release_artifacts.py validate-documentation-bundle --manifest release/sc-lint-release-checks.toml --publish-manifest release/publish-artifacts.toml --documentation-directory docs-bundle
ok: documentation bundle contains exactly the release manifest files
[...        1 lines, sha256 c02378c86adc5080] rc=0

$ cargo test -p sc-lint docs_ rc=0
test docs::tests::docs_paths_are_relative_to_the_bundle_and_read_only ... ok
test tests::docs_setup_alias_resolves_the_installation_guide ... ok
test tests::docs_command_parses_named_guides_and_path_mode ... ok
test docs::tests::published_docs_win_over_legacy_bundle_when_both_are_installed ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 120 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
```

The executable-relative lookup reads the staged bundle:

```text
docs configuration: header='Documentation root: <tmp>/k10-stage/bin/docs-bundle/configuration.md'; body.strip()==staged configuration.md.strip(): True
docs setup: header='Documentation root: <tmp>/k10-stage/bin/docs-bundle/installation.md'; body.strip()==staged installation.md.strip(): True
```

## Fragment links in changed Markdown (AC3)

```text
$ rg -n '\]\([^)]*#[^)]*\)' <19 changed .md files>
docs-bundle/ci.md:38:[detailed test-runner note](./best-practices.md#bundled-python-tests).
docs/requirements.md:595:  [ADR-017, changes 1 and compatibility](./sc-lint/adr/ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts).)
docs-bundle/using-sc-lint.md:92:[the detailed behavior notes](./best-practices.md#identity-literals-and-captured-output).
CHANGELOG.md:11:  in [troubleshooting](docs-bundle/troubleshooting.md#cargo-shear-exception-migration)
CHANGELOG.md:19:  in [troubleshooting](docs-bundle/troubleshooting.md#cargo-shear-exception-migration).
docs-bundle/best-practices.md:23:[cargo-shear migration guide](./troubleshooting.md#cargo-shear-exception-migration)
docs-bundle/best-practices.md:42:[CI guide](./ci.md#consumer-workflow) for the consumer commands.
docs-bundle/best-practices.md:64:[Just setup guide](./just-setup.md#generate-the-integration) describes the
docs-bundle/best-practices.md:84:[boundary package guide](./packages/sc-lint-boundary.md#canonical-package-identity)
docs-bundle/best-practices.md:88:[using sc-lint](./using-sc-lint.md#inspecting-output) for the consumer-facing
docs-bundle/just-setup.md:20:details are in [Best practices](./best-practices.md#template-rendering).
docs-bundle/installation.md:43:[the best-practices guide](./best-practices.md#binary-resolution-and-path-fallback).
docs/architecture.md:50:  (see [Backend Crate Isolation](#backend-crate-isolation))
docs/architecture.md:63:  (see [Boundary and Planning Data](#boundary-and-planning-data))
docs/architecture.md:65:  (see [Repo-local Automation](#repo-local-automation))
docs/architecture.md:80:  [Top-level CLI Role](#top-level-cli-role)), and the (d) kit justfile
docs/architecture.md:249:[Where the current code departs from the layering](#where-the-current-code-departs-from-the-layering)):
```

Each target heading was checked explicitly by matching the GitHub slug of
every `#` heading in the target file:

```text
PASS docs-bundle/ci.md:38 -> docs-bundle/best-practices.md#bundled-python-tests heading=### Bundled Python tests
PASS docs/requirements.md:595 -> docs/sc-lint/adr/ADR-017-phase-k-consumer-contracts.md#accepted-changes-to-existing-contracts heading=## Accepted changes to existing contracts
PASS docs-bundle/using-sc-lint.md:92 -> docs-bundle/best-practices.md#identity-literals-and-captured-output heading=### Identity literals and captured output
PASS CHANGELOG.md:11 -> docs-bundle/troubleshooting.md#cargo-shear-exception-migration heading=## Cargo-shear exception migration
PASS CHANGELOG.md:19 -> docs-bundle/troubleshooting.md#cargo-shear-exception-migration heading=## Cargo-shear exception migration
PASS docs-bundle/best-practices.md:23 -> docs-bundle/troubleshooting.md#cargo-shear-exception-migration heading=## Cargo-shear exception migration
PASS docs-bundle/best-practices.md:42 -> docs-bundle/ci.md#consumer-workflow heading=## Consumer workflow
PASS docs-bundle/best-practices.md:64 -> docs-bundle/just-setup.md#generate-the-integration heading=## Generate the integration
PASS docs-bundle/best-practices.md:84 -> docs-bundle/packages/sc-lint-boundary.md#canonical-package-identity heading=### Canonical package identity
PASS docs-bundle/best-practices.md:88 -> docs-bundle/using-sc-lint.md#inspecting-output heading=## Inspecting output
PASS docs-bundle/just-setup.md:20 -> docs-bundle/best-practices.md#template-rendering heading=### Template rendering
PASS docs-bundle/installation.md:43 -> docs-bundle/best-practices.md#binary-resolution-and-path-fallback heading=### Binary resolution and PATH fallback
PASS docs/architecture.md:50 -> docs/architecture.md#backend-crate-isolation heading=## Backend Crate Isolation
PASS docs/architecture.md:63 -> docs/architecture.md#boundary-and-planning-data heading=## Boundary and Planning Data
PASS docs/architecture.md:65 -> docs/architecture.md#repo-local-automation heading=## Repo-local Automation
PASS docs/architecture.md:80 -> docs/architecture.md#top-level-cli-role heading=## Top-level CLI Role
PASS docs/architecture.md:249 -> docs/architecture.md#where-the-current-code-departs-from-the-layering heading=### Where the current code departs from the layering
```

## Per-example input, entrypoint, output and status (AC2)

The driver (reproduced at the end) builds a disposable neutral workspace. Its
names come from the examples: `sample-app`, `store`, `core-api`, `db-driver`,
`app` and `fixture-kit`. It extracts every block verbatim from the **staged**
bundle by line range and runs each one through `$STAGED_SC_LINT`. The fixture
adds only two things: a one-record boundary inventory for `store`, which E4's
allowlist needs, and `sample-app`'s `serde_alias` path alias, which is E6's
documented key.

`setup` and `upgrade` run hermetically: `setup --dry-run` with
`SC_LINT_INSTALL_DIR` in the fixture, so nothing is downloaded. E3 runs the
documented lines with `/usr/local` replaced by a disposable prefix, against an
archive built locally with the declared layout
(`release/publish-artifacts.toml:91`, `share/sc-lint/sc-lint-docs`). E6 runs
the real `sc_lint.lint_cargo_shear` wrapper with a stub `cargo-shear` on
`PATH`, the same way the existing wrapper tests stub it.

```text
### E5.1 sc-lint init --just
entrypoint: <tmp>/k10-stage/bin/sc-lint init --just
status: rc=0
--- output (first 40 lines) ---
init: ok
--- (       1 lines) ---

note: user replaces the init-managed sc-lint.toml with the assembled config (configuration.md)
## assembled sc-lint.toml
# E1 configuration.md:5-16 (complete consumer config, verbatim)
[tool.sc-lint]
minimum_version = "0.7.3"

[[tool.sc-lint.lint]]
name = "rust"
command = ["cargo", "check", "--workspace"]

[[tool.sc-lint.test]]
name = "tests"
command = ["cargo", "test", "--workspace"]

# ASSEMBLY using-sc-lint.md:48-50: name the ci profile the E5/E7 commands select
[tool.sc-lint.lint_profiles]
ci = ["rust"]

# E2 configuration.md:66-79 (fragment, verbatim)
[tool.sc-lint.analyzers.portability]
fail_rules = ["PORT-001", "PORT-002"]
exclude_paths = ["tests/fixtures/**"]
unix_path_prefixes = ["/opt/"]

[tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]
"SCB-RUNTIME-001" = ["tests/fixtures/generated/**"]

[[tool.sc-lint.analyzers.portability.platform_adapters]]
unix_module = "src/platform/unix.rs"
windows_module = "src/platform/windows.rs"
reason = "These modules implement the same private adapter interface."

# E4 packages/sc-lint-boundary.md:106-123 (fragment, verbatim)
[[tool.sc-lint.analyzers.boundary.manifest_dependency_allowlists]]
owner_manifest_path = "crates/store/Cargo.toml"
allowed_dependencies = ["core-api", "db-driver"]

[[tool.sc-lint.analyzers.boundary.global_dependency_ownership]]
dependency = "db-driver"
allowed_manifest_paths = ["crates/store/Cargo.toml"]
allowed_source_roots = ["crates/store/src"]
manifest_message = "Only store may depend on db-driver."
source_message = "Only store source may import db-driver."

[[tool.sc-lint.analyzers.boundary.manifest_section_rules]]
owner_manifest_path = "crates/app/Cargo.toml"
dependency_package = "fixture-kit"
allowed_sections = ["dev-dependencies", "target.cfg(unix).dev-dependencies"]
message = "fixture-kit belongs in declared test sections."

toml parse sc-lint.toml: ['analyzers', 'lint', 'lint_profiles', 'minimum_version', 'test']
### E5.2 sc-lint setup (DRY RUN: hermetic, no download)
entrypoint: <tmp>/k10-stage/bin/sc-lint setup --dry-run
status: rc=0
--- output (first 40 lines) ---
a verified sc-lint release would be installed
--- (       1 lines) ---

### E5.3/E7.1 sc-lint lint ci --consumer --config sc-lint.toml --profile ci
entrypoint: <tmp>/k10-stage/bin/sc-lint lint ci --consumer --config sc-lint.toml --profile ci
status: rc=0
--- output (first 40 lines) ---
lint.ci.consumer: ok
--- (       1 lines) ---

### E5.4/E7.2 sc-lint test --config sc-lint.toml
entrypoint: <tmp>/k10-stage/bin/sc-lint test --config sc-lint.toml
status: rc=0
--- output (first 40 lines) ---
test: ok
--- (       1 lines) ---

### E5.5 sc-lint upgrade --check --config sc-lint.toml
entrypoint: <tmp>/k10-stage/bin/sc-lint upgrade --check --config sc-lint.toml
status: rc=0
--- output (first 40 lines) ---
a verified sc-lint release would be installed
--- (       1 lines) ---

### E5.6 sc-lint docs
entrypoint: <tmp>/k10-stage/bin/sc-lint docs
status: rc=0
--- output (first 40 lines) ---
Documentation root: <tmp>/k10-stage/bin/docs-bundle

# sc-lint operator manual

This is the offline, version-matched documentation bundle shipped with the
`sc-lint` release. It is usable without a source checkout or network access.
Start with [installation](./installation.md), then follow the
[Just setup guide](./just-setup.md) for the recommended consumer workflow.

## Operator guides

- [Installation](./installation.md) — Homebrew, release archives, and CI.
- [Using sc-lint](./using-sc-lint.md) — setup, lint, test, and daily commands.
- [Configuration](./configuration.md) — `sc-lint.toml`, profiles, and policy.
- [Just setup](./just-setup.md) — the canonical copyable consumer integration.
- [CI](./ci.md) — reproducible checks in continuous integration.
- [Upgrade](./upgrade.md) — safe version upgrades and rollback behavior.
- [Troubleshooting](./troubleshooting.md) — stable error codes and recovery.
- [Best practices](./best-practices.md) — agent, developer, and CI workflow.

## Package guides

Every published package has a guide, including library-only packages:

- [sc-lint](./packages/sc-lint.md)
- [sc-lint-attributes](./packages/sc-lint-attributes.md)
- [sc-lint-analyzer-support](./packages/sc-lint-analyzer-support.md)
- [sc-lint-boundary](./packages/sc-lint-boundary.md)
- [sc-lint-directives](./packages/sc-lint-directives.md)
- [sc-lint-portability](./packages/sc-lint-portability.md)
- [sc-lint-runtime](./packages/sc-lint-runtime.md)
- [sc-lint-schema](./packages/sc-lint-schema.md)

Use `sc-lint docs` to list this bundle, `sc-lint docs just-setup` to print a
guide, and `sc-lint docs --path` when an automation tool needs the installed
bundle directory.

--- (      37 lines) ---

### E2 portability fragment via assembled config
entrypoint: <tmp>/k10-stage/bin/sc-lint lint sc-portability --config sc-lint.toml
status: rc=0
--- output (first 40 lines) ---
sc-lint-portability: pass (0 findings)
--- (       1 lines) ---

### E2 runtime exclude fragment via assembled config
entrypoint: <tmp>/k10-stage/bin/sc-lint lint sc-runtime --config sc-lint.toml
status: rc=0
--- output (first 40 lines) ---
sc-lint-runtime: pass (0 findings)
--- (       1 lines) ---

### E4 boundary fragment via assembled config
entrypoint: <tmp>/k10-stage/bin/sc-lint lint sc-boundary --config sc-lint.toml
status: rc=0
--- output (first 40 lines) ---
sc-lint-boundary: pass (0 findings)
--- (       1 lines) ---

## E6 .just/lint-config.toml
[cargo_shear.allowed_unused_dependencies]
"sample-app.serde_alias" = "Used by generated serialization code."
### E6 cargo-shear wrapper accepts documented key
entrypoint: env PATH=<tmp>/k10-stub:$PATH PYTHONPATH=<worktree>/bindings/sc-lint-py/python python3 -m sc_lint.lint_cargo_shear --root <tmp>/k10-fx
status: rc=0
--- output (first 40 lines) ---
stub cargo-shear 
crates analyzed:
  crate        package      crate_path   manifest                     
  -----------  -----------  -----------  -----------------------------
  app          app          app          crates/app/Cargo.toml        
  core-api     core-api     core_api     crates/core-api/Cargo.toml   
  db-driver    db-driver    db_driver    crates/db-driver/Cargo.toml  
  fixture-kit  fixture-kit  fixture_kit  crates/fixture-kit/Cargo.toml
  store        store        store        crates/store/Cargo.toml      

--- (      10 lines) ---

### E6 negative control: package name instead of dependency key
entrypoint: env PATH=<tmp>/k10-stub:$PATH PYTHONPATH=<worktree>/bindings/sc-lint-py/python python3 -m sc_lint.lint_cargo_shear --root <tmp>/k10-fx
status: rc=2
--- output (first 40 lines) ---
SC_LINT_CARGO_SHEAR_CONFIG_INVALID field=cargo_shear.allowed_unused_dependencies key='sample-app.core-api' cause=dependency-key-not-found action="remove the stale key or use <package>.<dependency-key> from Cargo metadata"
--- (       1 lines) ---

## E3 executed script (prefix substituted)
tar -xzf sc-lint_0.7.3_x86_64-unknown-linux-gnu.tar.gz
install -d <tmp>/k10-prefix/bin <tmp>/k10-prefix/share/sc-lint
install -m 0755 sc-lint sc-lint-boundary sc-lint-portability sc-lint-runtime <tmp>/k10-prefix/bin/
cp -R share/sc-lint/sc-lint-docs <tmp>/k10-prefix/share/sc-lint/
sc-lint docs --path
### E3 archive install then docs --path
entrypoint: env PATH=<tmp>/k10-prefix/bin:$PATH bash -e <tmp>/k10-archive/install.sh
status: rc=0
--- output (first 40 lines) ---
Documentation root: <tmp>/k10-prefix/share/sc-lint/sc-lint-docs
--- (       1 lines) ---

### E4 negative control: app declares db-driver and fixture-kit as normal deps
entrypoint: <tmp>/k10-stage/bin/sc-lint lint sc-boundary --config sc-lint.toml
status: rc=10
--- output (first 40 lines) ---
sc-lint-boundary: fail (2 findings)
--- (       1 lines) ---

DONE
```

E4 negative-control findings (`sc-lint-boundary analyze --format json` on the
same copy). The documented messages appear verbatim:

```text
SCB-MANIFEST-004 manifest_dependency_owner_violation crates/app/Cargo.toml: Only store may depend on db-driver. (allowed manifests: crates/store/Cargo.toml)
SCB-MANIFEST-005 manifest_section_restriction crates/app/Cargo.toml [dependencies.fixture-kit]: fixture-kit belongs in declared test sections. (allowed sections: dev-dependencies, target.cfg(unix).dev-dependencies)
```

## Policy-location discrepancy (reconciled in f176a63)

`docs-bundle/configuration.md` said that boundary has no analyzer-specific keys
and that its manifest policy lives in the boundary inventory. That contradicted
REQ-PRODUCT-023, `packages/sc-lint-boundary.md` and the executed E4 example,
which reads the policy from `[tool.sc-lint.analyzers.boundary.*]`. Commit
`f176a63` corrects both passages: package-edge policy stays in inventory records,
and the three manifest dependency policy families are boundary analyzer keys.
No product defect was found.

## Driver

```bash
#!/usr/bin/env bash
# K10 D5 example verification driver. Examples are extracted verbatim from the
# STAGED docs-bundle; nothing is retyped. Usage: k10-run.sh <tmp>
set -u
T=$1; PYSRC=$2; STAGE=$T/k10-stage; export STAGED_SC_LINT=$STAGE/bin/sc-lint
DOCS=$STAGE/bin/docs-bundle; FX=$T/k10-fx; LOG=$T/k10-examples.log
rm -rf "$FX"; mkdir -p "$FX"; : > "$LOG"
block(){ awk -v s="$2" -v e="$3" 'NR>s && NR<e' "$DOCS/$1"; }   # body between fence lines s and e
rec(){ local id=$1; shift; echo "### $id" >>"$LOG"; echo "entrypoint: $*" >>"$LOG"; ( cd "${RUNDIR:-$FX}" && "$@" ) >"$T/k10-o.txt" 2>&1; local rc=$?
  echo "status: rc=$rc" >>"$LOG"; echo '--- output (first 40 lines) ---' >>"$LOG"; head -40 "$T/k10-o.txt" >>"$LOG"; echo "--- ($(wc -l <"$T/k10-o.txt") lines) ---" >>"$LOG"; echo >>"$LOG"; return $rc; }
# Neutral workspace: names come from the examples themselves.
mk(){ mkdir -p "$FX/crates/$1/src"; printf '[package]\nname = "%s"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nauthors.workspace = true\nlicense.workspace = true\nrepository.workspace = true\nhomepage.workspace = true\npublish = false\n\n%b' "$1" "$2" >"$FX/crates/$1/Cargo.toml"; body=${3:-"pub fn id() {}\n"}; printf '%b' "$body" >"$FX/crates/$1/src/lib.rs"; }
printf '[package]\nname = "sample-app"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nauthors.workspace = true\nlicense.workspace = true\nrepository.workspace = true\nhomepage.workspace = true\npublish = false\n\n[workspace]\nmembers = ["crates/*"]\nresolver = "2"\n\n[workspace.package]\nversion = "0.1.0"\nedition = "2021"\nrust-version = "1.85"\nauthors = ["fixture"]\nlicense = "MIT"\nrepository = "https://example.invalid"\nhomepage = "https://example.invalid"\n\n[dependencies]\nserde_alias = { package = \"core-api\", path = \"crates/core-api\", version = \"0.1.0\" }\n' >"$FX/Cargo.toml"
mkdir -p "$FX/src/platform"; printf '#[cfg(unix)]\n#[path = "platform/unix.rs"]\nmod platform;\n#[cfg(windows)]\n#[path = "platform/windows.rs"]\nmod platform;\npub fn name() -> &'"'"'static str { platform::name() }\n' >"$FX/src/lib.rs"
printf 'pub(crate) fn name() -> &'"'"'static str { "unix" }\n' >"$FX/src/platform/unix.rs"
printf 'pub(crate) fn name() -> &'"'"'static str { "windows" }\n' >"$FX/src/platform/windows.rs"
mk core-api ""; mk db-driver ""; mk fixture-kit ""
mk store '[dependencies]\ncore-api = { path = "../core-api", version = "0.1.0" }\ndb-driver = { path = "../db-driver", version = "0.1.0" }\n' 'pub use core_api::id;\npub use db_driver::id as driver;\n'
mk app '[dev-dependencies]\nfixture-kit = { path = "../fixture-kit", version = "0.1.0" }\n'
mkdir -p "$FX/boundaries/store"; printf '[planning]\ncurrent_sprint = "S.1"\n' >"$FX/boundaries/planning.toml"
cat >"$FX/boundaries/store/store.toml" <<'REC'
boundary_id = "BOUNDARY-Store"
owner_package = "store"
owner_crate_path = "store"
name = "Store"

[public]
facade = "Store"

[implementation]
type = "Store"
module = "store"
visibility = "public"
constructor = "none"

[composition]
roots = ["Store"]

[dependencies]
allowed_dependents = []
allowed_dependencies = ["core-api", "db-driver"]
forbidden_edges = []

[references]
scope = "outside_owner_crate"
forbidden = []

[testing]
allowed_test_double_paths = []
forbidden_test_bypasses = []

[enforcement]
lint_rules = []
review_gates = []

[status]
state = "concrete_landed"
REC
( cd "$FX" && git init -q && git add -A && git -c user.name=fx -c user.email=fx@invalid commit -qm fixture )
# E5 packages/sc-lint.md:23-30 and E7 using-sc-lint.md:43-46, verbatim argv via the staged path.
export SC_LINT_INSTALL_DIR=$FX/.managed
rec "E5.1 sc-lint init --just" "$STAGED_SC_LINT" init --just
echo "note: user replaces the init-managed sc-lint.toml with the assembled config (configuration.md)" >>"$LOG"
# Assemble one full sc-lint.toml from the labelled fragments.
{ echo "# E1 configuration.md:5-16 (complete consumer config, verbatim)"; block configuration.md 5 16
  echo; echo "# ASSEMBLY using-sc-lint.md:48-50: name the ci profile the E5/E7 commands select"
  printf '[tool.sc-lint.lint_profiles]\nci = ["rust"]\n'
  echo; echo "# E2 configuration.md:66-79 (fragment, verbatim)"; block configuration.md 66 79
  echo; echo "# E4 packages/sc-lint-boundary.md:106-123 (fragment, verbatim)"; block packages/sc-lint-boundary.md 106 123; } >"$FX/sc-lint.toml"
echo "## assembled sc-lint.toml" >>"$LOG"; cat "$FX/sc-lint.toml" >>"$LOG"; echo >>"$LOG"
python3 - "$FX/sc-lint.toml" "$DOCS/troubleshooting.md" >>"$LOG" 2>&1 <<'PY'
import sys,tomllib
print("toml parse sc-lint.toml:", sorted(tomllib.load(open(sys.argv[1],'rb'))["tool"]["sc-lint"]))
PY
rec "E5.2 sc-lint setup (DRY RUN: hermetic, no download)" "$STAGED_SC_LINT" setup --dry-run
rec "E5.3/E7.1 sc-lint lint ci --consumer --config sc-lint.toml --profile ci" "$STAGED_SC_LINT" lint ci --consumer --config sc-lint.toml --profile ci
rec "E5.4/E7.2 sc-lint test --config sc-lint.toml" "$STAGED_SC_LINT" test --config sc-lint.toml
rec "E5.5 sc-lint upgrade --check --config sc-lint.toml" "$STAGED_SC_LINT" upgrade --check --config sc-lint.toml
rec "E5.6 sc-lint docs" "$STAGED_SC_LINT" docs
rec "E2 portability fragment via assembled config" "$STAGED_SC_LINT" lint sc-portability --config sc-lint.toml
rec "E2 runtime exclude fragment via assembled config" "$STAGED_SC_LINT" lint sc-runtime --config sc-lint.toml
rec "E4 boundary fragment via assembled config" "$STAGED_SC_LINT" lint sc-boundary --config sc-lint.toml
# E6 troubleshooting.md:45-48 verbatim in .just/lint-config.toml; real wrapper, stub cargo-shear (as existing tests stub it).
mkdir -p "$FX/.just" "$T/k10-stub"; block troubleshooting.md 45 48 >"$FX/.just/lint-config.toml"
printf '#!/bin/sh\necho "stub cargo-shear $*" >&2\nexit 0\n' >"$T/k10-stub/cargo-shear"; chmod +x "$T/k10-stub/cargo-shear"
echo "## E6 .just/lint-config.toml" >>"$LOG"; cat "$FX/.just/lint-config.toml" >>"$LOG"
rec "E6 cargo-shear wrapper accepts documented key" env PATH="$T/k10-stub:$PATH" PYTHONPATH="$PYSRC" python3 -m sc_lint.lint_cargo_shear --root "$FX"
sed 's/"sample-app.serde_alias"/"sample-app.core-api"/' "$FX/.just/lint-config.toml" >"$T/k10-neg.toml"; cp "$FX/.just/lint-config.toml" "$T/k10-pos.toml"; cp "$T/k10-neg.toml" "$FX/.just/lint-config.toml"
rec "E6 negative control: package name instead of dependency key" env PATH="$T/k10-stub:$PATH" PYTHONPATH="$PYSRC" python3 -m sc_lint.lint_cargo_shear --root "$FX"
cp "$T/k10-pos.toml" "$FX/.just/lint-config.toml"
# E3 installation.md:31-37 verbatim except /usr/local -> disposable prefix; archive built locally with the declared layout.
AR=$T/k10-archive; PFX=$T/k10-prefix; rm -rf "$AR" "$PFX"; mkdir -p "$AR/pkg/share/sc-lint" "$PFX"
cp "$STAGE"/bin/sc-lint "$STAGE"/bin/sc-lint-boundary "$STAGE"/bin/sc-lint-portability "$STAGE"/bin/sc-lint-runtime "$AR/pkg/"
cp -R "$DOCS" "$AR/pkg/share/sc-lint/sc-lint-docs"
( cd "$AR/pkg" && tar -czf ../sc-lint_0.7.3_x86_64-unknown-linux-gnu.tar.gz . ); mkdir -p "$AR/work"; cp "$AR/sc-lint_0.7.3_x86_64-unknown-linux-gnu.tar.gz" "$AR/work/"
block installation.md 31 37 | sed "s#/usr/local#$PFX#g" >"$AR/install.sh"
echo "## E3 executed script (prefix substituted)" >>"$LOG"; cat "$AR/install.sh" >>"$LOG"
RUNDIR="$AR/work" rec "E3 archive install then docs --path" env PATH="$PFX/bin:$PATH" bash -e "$AR/install.sh"
# E4 negative control on a copy: app takes db-driver and fixture-kit as normal dependencies.
NEG=$T/k10-fx-neg; rm -rf "$NEG"; cp -R "$FX" "$NEG"
printf '\n[dependencies]\ndb-driver = { path = "../db-driver", version = "0.1.0" }\nfixture-kit = { path = "../fixture-kit", version = "0.1.0" }\n' >>"$NEG/crates/app/Cargo.toml"
RUNDIR="$NEG" rec "E4 negative control: app declares db-driver and fixture-kit as normal deps" "$STAGED_SC_LINT" lint sc-boundary --config sc-lint.toml
echo DONE >>"$LOG"
```
