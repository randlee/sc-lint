# sc-lint v0.8.8 — Declared Visibility Is Now Checked, Dead Config Is Rejected

**Released:** November 11, 2026 · **Install:** `brew install randlee/tap/sc-lint` (Homebrew), `pip install sc-lint` (PyPI), `cargo install sc-lint` (crates.io), or the platform archives on the [release page](https://github.com/randlee/sc-lint/releases/tag/v0.8.8)

[Changelog](https://github.com/randlee/sc-lint/blob/main/CHANGELOG.md) · [Release notes](https://github.com/randlee/sc-lint/releases/tag/v0.8.8) · [Operator manual](https://github.com/randlee/sc-lint/blob/main/docs-bundle/README.md)

---

## Architecture-Boundary Enforcer

**As a maintainer, I want sc-lint to enforce architecture boundaries for agents, so that agent-authored changes stay inside the intended module/crate boundaries instead of leaking across layers.**

Boundary records have always been able to *declare* an implementation's visibility — `private`, `pub(crate)` — but nothing read the declaration. v0.8.8 closes that gap with `SCB-BOUNDARY-005`. For a record with `[implementation].visibility`, the named source type in `[implementation].module` is compared against the declaration, and a type wider than declared fails. When `[implementation].constructor` is present it is compared with that type's inherent `fn new`, so `private` no longer permits a public constructor. A record that names an item which cannot be found is reported rather than silently passing, and `trait_only` records report a public implementing type. Findings carry the `declared_visibility_mismatch` kind. The same diff resolves impl owners through imports and inherent impl blocks, so a constructor reached through a `use` is matched to its declared type instead of producing a false finding.

This rule is a real guardrail for the pattern it covers: an agent that widens a type's visibility to make a compile error go away now gets a finding at the declaration site instead of a boundary violation discovered by review. Repositories that run with an explicit `fail_rules` list must add `SCB-BOUNDARY-005` to it — the rule is reported but not gating until listed, and a default-configured repository gains it as gating immediately. `SCB-BOUNDARY-005` was introduced across the 0.8.7 pre-release line and ships stable here for the first time.

One removal is breaking. The boundary record `[enforcement]` section (`lint_rules`, `review_gates`) is gone: no check ever read it, so it was configuring nothing while implying it configured something. A record that still declares it no longer loads — inventory loading fails with a diagnostic naming the offending file. Delete the section from every boundary record you own; behavior does not change, because the section never had any.

---

## Agent Orchestrator

**As an orchestrator, I want sc-lint to keep agent-authored code conformant to shared conventions, so that parallel agents' output stays consistent without per-agent review.**

Fleet-wide version drift gets one more detector. Version sync now reports `python.dependency_specifier` when a project listed in `[version_sync.python].python_manifests` depends on another listed project with a specifier that does not admit the workspace version — the stale `atm-graft>=1.4,<1.5` left behind after a bump. `expected` is the workspace version and `actual` is the offending specifier, so the finding is actionable from JSON alone. The wheel now depends on `packaging` to do the comparison. Roll this out with the `fail_rules` note above: a repo with an explicit list picks up neither the new boundary rule nor any changed gating until the rule ID is added, while default-configured repos get the detection on upgrade.

---

## Agent Team Member

**As an agent on an SC team, I want excellent, actionable error messages, so that I can self-correct my code without a human pointing at the mistake.**

The breaking removal is the one that matters to you: a boundary record carrying a leftover `[enforcement]` table now fails to load with a message naming the file, which tells you exactly which record to edit instead of leaving you to guess why the inventory stopped loading. Installer backend probes also now honor the configured timeout instead of a built-in default, so a slow Python or Cargo backend reports the deadline you set rather than failing on someone else's.

---

## Upgrade Notes

Remove `[enforcement]` from every boundary record before upgrading — records that still declare it fail to load. If your repository sets an explicit `fail_rules` list, add `SCB-BOUNDARY-005` to make declared-visibility checking gating, and expect new `python.dependency_specifier` findings in workspaces whose Python manifests pin each other. Neither change requires a config rewrite beyond those two edits; 0.8.7 was a pre-release only and has no separate stable announcement.

---

*sc-lint is the shared lint and conformance layer for the Synaptic Canvas repositories.*
