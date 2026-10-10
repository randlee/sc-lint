---
status: complete
branch: feature/documentation-bundle
worktree: /Users/randlee/github/sc-publish-worktrees/feature/documentation-bundle
---

# FEATURE: Optional `[documentation_bundle]` release-archive contract

## Goal

Let a consumer declare a documentation directory that is validated, staged into
every release archive, and verified in the produced archive, without any
consumer-specific logic in the kit. Tracking bead: lint-spx.43 (sc-lint).

## Deliverables

1. `release_documentation.py`: manifest loader/validator, `stage`, source
   `validate`, and `validate-release-archive` (tar.gz and zip), registered as
   `release_artifacts.py` subcommands (kept out of `release_artifacts.py` to
   hold its line ceiling).
2. `release_manifest.py` passes the optional table through; `release-package-config`
   includes it only when declared.
3. `release.yml` and `prerelease-archive.yml` packaging steps stage the bundle
   and validate each archive (identical logic; parity test preserved).
4. `install.py` and `publish-artifacts.toml.j2` accept and render the optional
   `documentation_bundle`; absent input renders no table.
5. Tests: `test_documentation_bundle.py` (declared and absent) plus script-list
   updates; README section "Optional documentation bundle".

## Notes

- No behavior change when the table is absent.
- Archive layout is unchanged: the bundle lands at
  `<archive-root>/<package_directory>/`.
- sc-publish has no `docs/project-plan.md`; the sprint doc is the only index.
