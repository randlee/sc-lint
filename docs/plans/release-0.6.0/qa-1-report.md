# QA Findings Update

Generated: 2026-10-10T15:49Z
QA Pass: QA-1
Sprint/Task: lint-spx.42 / 0.6.0 release notes / lint-spx.44
Branch: `chore/sc-publish-wheel-release`
Commit: `0c3c3a7`
PR: #208
Verdict: **FAIL**

## Machine Status (JSON)
```json
{
  "sprint": "lint-spx.42 / 0.6.0 release notes",
  "task": "lint-spx.44",
  "branch": "chore/sc-publish-wheel-release",
  "commit": "0c3c3a7",
  "pr": 208,
  "verdict": "FAIL",
  "findings": {
    "blocking": 0,
    "important": 2,
    "minor": 2
  },
  "blocking_ids": [],
  "merge_readiness": "NOT READY",
  "merge_reason": "2 Important findings open (manylinux_2_39 wheel tag; stale release-notes validation head)",
  "next_action": "Fix QA-R1, QA-R2 (and optionally QA-M1, QA-M2) on PR #208, then re-dispatch QA-2 scoped to these ids",
  "owner": "team-lead"
}
```

## Findings Summary
- Blocking: 0
- Important: 2
- Minor: 2

## Blocking Findings
None.

## Detailed Findings
### Scope 1: PR #208 (wheel publish proof, `docs/release-pypi.md`, pyproject metadata)

Verified accurate against the workflows: `release.yml` `build-python-wheels` (:370-447, sdist on linux only, version check in each job), `release` needs `publish-crates` (:449-450), `publish-pypi` needs `release`, env `pypi`, `PYPI_TOKEN`, `maturin upload --skip-existing` (:485-513); `pypi-publish.yml` inputs `tag`/`target`, reuses release assets, never rebuilds; `release-preflight.yml` `testpypi-rehearsal` env `testpypi`, `TEST_PYPI_TOKEN`, `--skip-existing` (:183-222); `[[python_distributions]]` in `release/publish-artifacts.toml:135-142`; `SC_LINT_WHEEL_DIR` and `--no-index` in `packages/sc-lint-adoption/.sc-lint/bootstrap:69-96`. I built the sdist locally (`sc_lint-0.6.0.tar.gz`, PKG-INFO name/version/classifiers/requires-python correct). `twine` is not installed on this host, so `twine check` was not re-run; the PR body reports it passed. Nothing was uploaded or tagged.

**QA-R1 (Important)** Linux wheel is tagged `manylinux_2_39`. The last Release Preflight run (35559822647, 2026-09-21) built `sc_lint-0.6.0-cp39-abi3-manylinux_2_39_x86_64.whl` because `maturin build` runs on `ubuntu-latest` with no compatibility setting (`release.yml:413`, `release-preflight.yml:210`; no `manylinux`/`compatibility` anywhere in `.github` or `bindings/sc-lint-py`). That wheel installs only on glibc >= 2.39 (Ubuntu 24.04+). Older consumer hosts (Ubuntu 22.04, Debian 12) get no matching wheel and pip falls back to the sdist, which needs a Rust toolchain; the offline `SC_LINT_WHEEL_DIR` path has no fallback at all. PyPI acceptance of this tag is also unverified. Neither the sprint doc nor `docs/release-pypi.md` mentions it. Fix: build the Linux wheel in a manylinux2014 container (or zig) or document the glibc floor as an accepted limit, and state it in `docs/release-pypi.md`.

**QA-M1 (Minor)** The sdist ships no LICENSE file while PKG-INFO says `License: MIT` (PR body already notes this as non-blocking). Add `license-files` or a LICENSE in `bindings/sc-lint-py`.

### Scope 2: `release/RELEASE-NOTES-0.6.0.md` (merged via #189)

Cited PRs checked against GitHub: every cited number is a real merged PR; #85 and #88 are issues, as the notes say. The `0.2.0` schema line at `crates/sc-lint-boundary/src/lib.rs:42` is correct. The `wait_after_publish_seconds = 15` claim holds for analyzer-support's neighbours in `release/publish-artifacts.toml`.

**QA-R2 (Important)** `RELEASE-NOTES-0.6.0.md:90-91` says `develop` head is `e2477c38` and the latest develop CI run is `35559810599` (2026-09-21). `origin/develop` is `66dbca97`; `e2477c38` is a 2026-09-20 ancestor, so the Validation section is stale by the 0.6.0 bump (#148), #189, #205, #207 and the wheel layer, while the header (`:6`) is dated 2026-10-09. Refresh the head SHA and CI run at release time, or reword the section to name the commit it was validated at.

**QA-M2 (Minor)** PRs merged to develop since v0.5.0 (2026-08-13 16:29 PT) that are neither cited nor justified: #130 (Phase G plan), #147 (integrate/phase-G -> develop), #148 (bump to 0.6.0), #151 (merge main forward), #112 and #113 (post-0.5.0 back-propagation/template recovery), #189, #205 and #207 (chore skills). Cite #147/#148 under the release and add one line that #112/#113/#151/#189/#205/#207/#130 are internal.

CI on #208 at 0c3c3a7: 12/12 pass. Stack: single layer, base develop. No sc-publish review performed.

## Merge Readiness
- Status: **NOT READY**
- Reason: 2 Important findings open (manylinux_2_39 wheel tag; stale release-notes validation head)

## Next Action
- Action: Fix QA-R1, QA-R2 (and optionally QA-M1, QA-M2) on PR #208, then re-dispatch QA-2 scoped to these ids
- Owner: team-lead
