//! The finite managed GitHub workflow transformer.
//!
//! This module is deliberately not a general workflow editor.  It recognizes
//! exactly the generated workflow below; every other existing file is kept
//! untouched and reported as a reviewed manual conflict.

use std::path::{Path, PathBuf};

use serde_json::json;
use sha2::Digest;

use crate::CliError;
use crate::consts::CLI_CONFIGURE_UNMANAGED_COLLISION;

use super::artifact::{ArtifactKind, ManagedArtifact};
use super::reviewed_removals::{configure_apply_error, plan_proposes};

pub(crate) const WORKFLOW_PATH: &str = ".github/workflows/sc-lint.yml";

const CANONICAL_WORKFLOW: &str = concat!(
    "name: sc-lint\n",
    "\n",
    "on:\n",
    "  pull_request:\n",
    "\n",
    "permissions:\n",
    "  contents: read\n",
    "\n",
    "jobs:\n",
    "  sc-lint:\n",
    "    runs-on: ubuntu-latest\n",
    "    steps:\n",
    "      - uses: randlee/sc-lint@v1\n",
    "        with:\n",
    "          operation: setup\n",
    "          config-path: sc-lint.toml\n",
    "      - uses: randlee/sc-lint@v1\n",
    "        with:\n",
    "          operation: lint\n",
    "          config-path: sc-lint.toml\n",
    "      - uses: randlee/sc-lint@v1\n",
    "        with:\n",
    "          operation: test\n",
    "          config-path: sc-lint.toml\n",
);

/// A byte-deterministic, validated product-owned workflow output.
pub(crate) struct WorkflowYamlArtifact {
    target: PathBuf,
    bytes: Vec<u8>,
}

impl WorkflowYamlArtifact {
    pub(crate) fn canonical(target: PathBuf) -> Self {
        Self {
            target,
            bytes: CANONICAL_WORKFLOW.as_bytes().to_vec(),
        }
    }

    #[cfg(test)]
    fn from_bytes(target: PathBuf, bytes: Vec<u8>) -> Self {
        Self { target, bytes }
    }
}

impl ManagedArtifact for WorkflowYamlArtifact {
    fn kind(&self) -> ArtifactKind {
        ArtifactKind::WorkflowYaml
    }

    fn target(&self) -> &Path {
        &self.target
    }

    fn staged_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn validate_staged(&self) -> Result<(), CliError> {
        let value =
            serde_yaml_ng::from_slice::<serde_yaml_ng::Value>(&self.bytes).map_err(|error| {
                configure_apply_error(
                    CLI_CONFIGURE_UNMANAGED_COLLISION,
                    "the generated workflow is not valid YAML",
                    error.to_string(),
                    "Review the exportable patch; no repository files were changed.",
                )
            })?;
        if !value.is_mapping() {
            return Err(configure_apply_error(
                CLI_CONFIGURE_UNMANAGED_COLLISION,
                "the generated workflow is not a YAML mapping",
                "a GitHub workflow must have a mapping as its document root",
                "Review the exportable patch; no repository files were changed.",
            ));
        }
        Ok(())
    }
}

/// Add the workflow only when the reviewed plan selected this finite output.
/// An exact canonical fingerprint is already current; every other existing
/// shape remains user-owned and is never modified.
#[expect(
    clippy::result_large_err,
    reason = "Workflow conflicts retain the configure recovery envelope."
)]
pub(crate) fn add_workflow_artifact(
    root: &Path,
    plan: &serde_json::Value,
    artifacts: &mut Vec<Box<dyn ManagedArtifact>>,
) -> Result<(), CliError> {
    if !plan_proposes(plan, WORKFLOW_PATH) {
        return Ok(());
    }
    let target = root.join(WORKFLOW_PATH);
    match std::fs::read(&target) {
        Ok(existing) if existing == CANONICAL_WORKFLOW.as_bytes() => Ok(()),
        Ok(existing) => Err(manual_conflict(&target, &existing)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            artifacts.push(Box::new(WorkflowYamlArtifact::canonical(target)));
            Ok(())
        }
        Err(error) => Err(configure_apply_error(
            CLI_CONFIGURE_UNMANAGED_COLLISION,
            "the managed workflow target could not be read",
            error.to_string(),
            "Check repository permissions and review the exportable patch before retrying.",
        )),
    }
}

fn manual_conflict(target: &Path, existing: &[u8]) -> CliError {
    let observed_digest = format!("sha256:{:x}", sha2::Sha256::digest(existing));
    configure_apply_error(
        CLI_CONFIGURE_UNMANAGED_COLLISION,
        "an existing workflow is not an exact sc-lint managed fingerprint",
        format!("`{}` has digest {observed_digest}", target.display()),
        "Review the exportable patch; sc-lint will not overwrite the existing workflow.",
    )
    .with_detail(
        "manual_conflict",
        json!({
            "operation_id": "github-workflow",
            "path": WORKFLOW_PATH,
            "kind": "manual_conflict",
            "conflict": {
                "code": CLI_CONFIGURE_UNMANAGED_COLLISION,
                "observed_digest": observed_digest,
                "recovery": "review_exported_patch"
            },
            "exportable_patch": {
                "format": "unified-diff",
                "path": WORKFLOW_PATH,
                "content": exportable_patch()
            }
        }),
    )
}

fn exportable_patch() -> String {
    let body = CANONICAL_WORKFLOW
        .lines()
        .map(|line| format!("+{line}\n"))
        .collect::<String>();
    format!(
        "--- /dev/null\n+++ b/{WORKFLOW_PATH}\n@@ -0,0 +1,{} @@\n{body}",
        CANONICAL_WORKFLOW.lines().count()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configure::apply;

    #[test]
    fn canonical_workflow_parses_as_yaml_and_has_the_required_action_steps() {
        let root = tempfile::tempdir().expect("temp root");
        let artifact = WorkflowYamlArtifact::canonical(root.path().join(WORKFLOW_PATH));
        artifact.validate_staged().expect("canonical YAML");
        let source = std::str::from_utf8(artifact.staged_bytes()).expect("utf8");
        assert_eq!(source.matches("uses: randlee/sc-lint@v1").count(), 3);
        for operation in ["setup", "lint", "test"] {
            assert!(source.contains(&format!("operation: {operation}")));
        }
        assert!(!source.contains("actions/checkout"));
    }

    #[test]
    fn malformed_yaml_is_rejected_before_commit() {
        let root = tempfile::tempdir().expect("temp root");
        let target = root.path().join(WORKFLOW_PATH);
        let artifact = WorkflowYamlArtifact::from_bytes(target.clone(), b"jobs: [\n".to_vec());
        assert!(artifact.validate_staged().is_err());
        assert!(!target.exists());
    }

    #[test]
    fn exact_fingerprint_reapplies_without_an_artifact() {
        let root = tempfile::tempdir().expect("temp root");
        let target = root.path().join(WORKFLOW_PATH);
        std::fs::create_dir_all(target.parent().expect("parent")).expect("workflow directory");
        std::fs::write(&target, CANONICAL_WORKFLOW).expect("canonical workflow");
        let plan =
            serde_json::json!({"operations": [{"path": WORKFLOW_PATH, "kind": "propose_create"}]});
        let mut artifacts = Vec::new();
        add_workflow_artifact(root.path(), &plan, &mut artifacts).expect("idempotent");
        assert!(artifacts.is_empty());
    }

    #[test]
    fn unknown_and_near_match_workflows_are_manual_conflicts_without_writes() {
        for contents in [
            "name: user workflow\n".to_string(),
            CANONICAL_WORKFLOW.replace("operation: lint", "operation: full-lint"),
        ] {
            let root = tempfile::tempdir().expect("temp root");
            let target = root.path().join(WORKFLOW_PATH);
            std::fs::create_dir_all(target.parent().expect("parent")).expect("workflow directory");
            std::fs::write(&target, contents).expect("existing workflow");
            let before = std::fs::read(&target).expect("before");
            let plan = serde_json::json!({"operations": [{"path": WORKFLOW_PATH, "kind": "propose_create"}]});
            let error =
                add_workflow_artifact(root.path(), &plan, &mut Vec::new()).expect_err("conflict");
            assert_eq!(error.code(), CLI_CONFIGURE_UNMANAGED_COLLISION);
            assert_eq!(error.details["manual_conflict"]["kind"], "manual_conflict");
            assert_eq!(std::fs::read(&target).expect("unchanged"), before);
        }
    }

    #[test]
    fn real_workflow_artifact_rolls_back_with_a_later_extension_failure() {
        let root = tempfile::tempdir().expect("temp root");
        let workflow = root.path().join(WORKFLOW_PATH);
        let result = apply::commit_with_post_commit_failure_for_test(
            vec![
                Box::new(super::WorkflowYamlArtifact::canonical(workflow.clone())),
                Box::new(super::super::artifact::BytesArtifact::new(
                    super::super::artifact::ArtifactKind::Json,
                    root.path().join("later.json"),
                    b"{}\n".to_vec(),
                )),
            ],
            1,
        );
        assert_eq!(
            result.expect_err("rollback").code(),
            "CLI.CONFIGURE_COMMIT_FAILED"
        );
        assert!(!workflow.exists());
    }
}
