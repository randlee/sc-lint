---
sprint: lint-spx.42
bead: lint-spx.42
epic: lint-spx
status: complete
branch: chore/sc-publish-wheel-release
worktree: /Users/randlee/github/sc-lint-worktrees/chore/sc-publish-wheel-release
pr_target: develop
parent: develop @ c863d177a
closure_type: release
adrs: []
requirements: []
---

# lint-spx.42 — Prove the sc-lint wheel publishes through the existing pipeline

Rescoped by Rand: 0.6.0 publishes the `sc-lint` wheel (`bindings/sc-lint-py`)
through the existing `release.yml`, `pypi-publish.yml` and
`release-preflight.yml`. The sc-publish kit install moved to `lint-3x2.3`
(local WIP saved on branch `chore/sc-publish-kit-adoption`, not pushed).

## Why

The adoption kit bootstrap runs `pip install sc-lint==<version>` from PyPI
(`packages/sc-lint-adoption/.sc-lint/bootstrap`). The wheel was never
published, so 0.6.0 cannot be adopted without it.

## Deliverables

1. Prove the path short of uploading: local `maturin build` and `maturin
   sdist`, `twine check`, clean-venv install, and the kit bootstrap with
   `SC_LINT_WHEEL_DIR`; review the three workflows' PyPI jobs and ordering.
2. Check PyPI-facing metadata in `bindings/sc-lint-py/pyproject.toml`.
3. Fix anything blocking the publish with a minimal diff.
4. Write `docs/release-pypi.md` (environments, secrets, trusted publishing,
   first publish, re-run).

## Acceptance Criteria

- `just lint` and `just test` pass; PR CI green.
- Nothing is uploaded, tagged or released by this task.
