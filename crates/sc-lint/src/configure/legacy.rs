//! The finite F.4a deletion allowlist for one exact sc-compose 0.4 bundle.
//!
//! The planner proves the content digests; apply independently proves that a
//! reviewed `propose_remove` still names one of these paths and carries the
//! only reason that authorizes a destructive configure operation.

use crate::configure::artifact::ArtifactKind;

pub(crate) const EXACT_SC_COMPOSE_04_REASON: &str = "exact_sc_compose_0_4_legacy_fingerprint";

const WORKFLOW_PATHS: &[&str] = &[
    ".github/actions/setup-sc-lint/action.yml",
    ".github/actions/setup-lint-toolchain/action.yml",
];
const TOML_PATHS: &[&str] = &[".just/lint-config.toml"];
const SCRIPT_PATHS: &[&str] = &[
    "scripts/materialize_sc_lint_runtime.py",
    ".just/.sc-lint-runtime-version",
    ".just/check_version_sync.py",
    ".just/fixture_constants.py",
    ".just/lint_boundaries.py",
    ".just/lint_cargo_deny.py",
    ".just/lint_cargo_modules.py",
    ".just/lint_cargo_shear.py",
    ".just/lint_codespell.py",
    ".just/lint_common.py",
    ".just/lint_identity_literals.py",
    ".just/lint_line_counts.py",
    ".just/lint_manifests.py",
    ".just/lint_sc_boundary.py",
    ".just/lint_sc_portability.py",
    ".just/print_help.py",
    ".just/python_adapter.py",
    ".just/run_fmt.py",
    ".just/run_lint.py",
    ".just/run_pytests.py",
    ".just/run_version.py",
    ".just/view_common.py",
    ".just/view_findings.py",
];

/// Return a concrete transaction kind only for the complete-plan allowlist.
/// Path matching is deliberately finite rather than a `.just/**` glob so a
/// user-owned file with a familiar name never becomes removable.
pub(crate) fn allowlisted_removal_kind(path: &str, reason: Option<&str>) -> Option<ArtifactKind> {
    if reason != Some(EXACT_SC_COMPOSE_04_REASON) {
        return None;
    }
    if WORKFLOW_PATHS.contains(&path) {
        return Some(ArtifactKind::WorkflowYaml);
    }
    if TOML_PATHS.contains(&path) {
        return Some(ArtifactKind::Toml);
    }
    SCRIPT_PATHS.contains(&path).then_some(ArtifactKind::Shell)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_exact_fingerprint_paths_and_reason_are_allowlisted() {
        assert_eq!(
            allowlisted_removal_kind(
                ".github/actions/setup-sc-lint/action.yml",
                Some(EXACT_SC_COMPOSE_04_REASON),
            ),
            Some(ArtifactKind::WorkflowYaml)
        );
        assert_eq!(
            allowlisted_removal_kind(".just/lint-config.toml", Some(EXACT_SC_COMPOSE_04_REASON)),
            Some(ArtifactKind::Toml)
        );
        assert_eq!(
            allowlisted_removal_kind(".just/not-owned.py", Some(EXACT_SC_COMPOSE_04_REASON)),
            None
        );
        assert_eq!(allowlisted_removal_kind(".just/run_lint.py", None), None);
    }
}
