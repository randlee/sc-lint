//! Helpers for applying a reviewed configure plan.
//!
//! Keeping plan validation and the finite legacy-removal allowlist outside the
//! CLI dispatch module makes the command surface small without changing the
//! configure transaction or its structured recovery contract.

use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::CliError;
use crate::consts::CLI_CONFIGURE_UNMANAGED_COLLISION;

use super::artifact::{ArtifactKind, BytesArtifact, ManagedArtifact, RemoveArtifact};

#[expect(
    clippy::result_large_err,
    reason = "Configure request rereads preserve structured stale-plan recovery."
)]
pub(crate) fn read_configure_request(path: &Path) -> Result<Value, CliError> {
    if path == Path::new("-") {
        return Err(configure_apply_error(
            "CLI.CONFIGURE_STALE_PLAN",
            "configure --apply cannot reread a request from standard input",
            "the reviewed request must be reproducible for stale-plan protection",
            "Save the request JSON to a file and rerun configure --apply.",
        ));
    }
    let raw = std::fs::read(path).map_err(|error| {
        configure_apply_error(
            "CLI.CONFIGURE_STALE_PLAN",
            "the configure request could not be reread",
            error.to_string(),
            "Restore the reviewed request file, then regenerate and review the plan.",
        )
    })?;
    serde_json::from_slice(&raw).map_err(|error| {
        configure_apply_error(
            "CLI.CONFIGURE_STALE_PLAN",
            "the configure request is no longer valid JSON",
            error.to_string(),
            "Restore the reviewed request file, then regenerate and review the plan.",
        )
    })
}

#[expect(
    clippy::result_large_err,
    reason = "Stale reviewed plans require the frozen configure recovery envelope."
)]
pub(crate) fn ensure_reviewed_plan_matches(
    reviewed: &Value,
    fresh: &Value,
) -> Result<(), CliError> {
    let same_identifier = reviewed.get("plan_id") == fresh.get("plan_id");
    let same_preconditions = reviewed.get("preconditions") == fresh.get("preconditions");
    if same_identifier && same_preconditions {
        return Ok(());
    }
    let changed_path = first_changed_precondition(reviewed, fresh)
        .unwrap_or_else(|| "the configure request".to_string());
    Err(configure_apply_error(
        "CLI.CONFIGURE_STALE_PLAN",
        "the reviewed configure plan is stale",
        format!("`{changed_path}` or the requested configuration changed after planning"),
        "Regenerate the plan, review the updated changes, then rerun configure --apply.",
    )
    .with_detail("path", serde_json::json!(changed_path)))
}

fn first_changed_precondition(reviewed: &Value, fresh: &Value) -> Option<String> {
    let reviewed_conditions = reviewed.get("preconditions")?.as_array()?;
    let fresh_conditions = fresh.get("preconditions")?.as_array()?;
    reviewed_conditions
        .iter()
        .zip(fresh_conditions)
        .find_map(|(before, after)| {
            (before != after).then(|| {
                before
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string()
            })
        })
}

#[expect(
    clippy::result_large_err,
    reason = "Unresolved conflicts require the frozen configure recovery envelope."
)]
pub(crate) fn ensure_applyable_plan(plan: &Value) -> Result<(), CliError> {
    let conflicts = plan
        .get("conflicts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let manual_steps = plan
        .get("manual_steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if conflicts.is_empty() && manual_steps.is_empty() {
        return Ok(());
    }
    Err(configure_apply_error(
        CLI_CONFIGURE_UNMANAGED_COLLISION,
        "the reviewed configure plan contains unresolved conflicts",
        "apply will not bypass conflicts or manual review steps",
        "Review the exportable patch or select a non-conflicting configuration, then regenerate the plan.",
    ))
}

pub(crate) fn plan_proposes(plan: &Value, path: &str) -> bool {
    plan.get("operations")
        .and_then(Value::as_array)
        .is_some_and(|operations| {
            operations.iter().any(|operation| {
                operation.get("path").and_then(Value::as_str) == Some(path)
                    && operation.get("kind").and_then(Value::as_str) == Some("propose_create")
            })
        })
}

#[expect(
    clippy::result_large_err,
    reason = "Reviewed removal paths retain the configure transaction recovery envelope."
)]
pub(crate) fn add_reviewed_removals(
    root: &Path,
    plan: &Value,
    artifacts: &mut Vec<Box<dyn ManagedArtifact>>,
) -> Result<(), CliError> {
    let Some(operations) = plan.get("operations").and_then(Value::as_array) else {
        return Ok(());
    };
    for operation in operations {
        if operation.get("kind").and_then(Value::as_str) != Some("propose_remove") {
            continue;
        }
        let relative = operation
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                configure_apply_error(
                    CLI_CONFIGURE_UNMANAGED_COLLISION,
                    "the reviewed removal operation has no path",
                    "the configure plan violates the v1 operation contract",
                    "Regenerate and review the configure plan.",
                )
            })?;
        let kind = crate::configure::legacy::allowlisted_removal_kind(
            relative,
            operation.get("reason").and_then(Value::as_str),
        )
        .ok_or_else(|| {
            configure_apply_error(
                CLI_CONFIGURE_UNMANAGED_COLLISION,
                "the reviewed removal operation is not in the sc-lint legacy allowlist",
                format!("`{relative}` is not an exact, fingerprint-authorized legacy artifact"),
                "Regenerate and review the configure plan; do not remove consumer-owned files automatically.",
            )
        })?;
        let relative_path = Path::new(relative);
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(configure_apply_error(
                CLI_CONFIGURE_UNMANAGED_COLLISION,
                "the reviewed removal path escapes the consumer root",
                format!("`{relative}` is not a repository-relative managed path"),
                "Regenerate and review the configure plan.",
            ));
        }
        artifacts.push(Box::new(RemoveArtifact::new(
            kind,
            root.join(relative_path),
        )));
    }
    Ok(())
}

#[expect(
    clippy::result_large_err,
    reason = "Just coexistence failures require structured configure recovery."
)]
pub(crate) fn add_just_artifacts(
    root: &Path,
    plan: &Value,
    artifacts: &mut Vec<Box<dyn ManagedArtifact>>,
) -> Result<(), CliError> {
    let root_justfile = root.join("Justfile");
    if root_justfile.exists() {
        let existing = std::fs::read_to_string(&root_justfile).map_err(|error| {
            configure_apply_error(
                CLI_CONFIGURE_UNMANAGED_COLLISION,
                "the existing Justfile could not be read",
                error.to_string(),
                "Check Justfile permissions and review the exportable patch before retrying.",
            )
        })?;
        reject_reserved_recipes(&existing)?;
        let updated = crate::configure::just::insert_or_replace(&existing)?;
        artifacts.push(Box::new(BytesArtifact::new(
            ArtifactKind::Justfile,
            root_justfile,
            updated.into_bytes(),
        )));
    } else {
        artifacts.push(Box::new(BytesArtifact::new(
            ArtifactKind::Justfile,
            root_justfile,
            crate::consumer_integration::canonical_consumer_justfile().into_bytes(),
        )));
    }
    if plan_proposes(plan, ".sc-lint/justfile") {
        add_managed_creation(
            artifacts,
            ArtifactKind::Justfile,
            root.join(".sc-lint/justfile"),
            crate::consumer_integration::canonical_consumer_justfile().into_bytes(),
        )?;
    }
    Ok(())
}

#[expect(
    clippy::result_large_err,
    reason = "Managed-file drift is a structured no-write configure collision."
)]
pub(crate) fn add_managed_creation(
    artifacts: &mut Vec<Box<dyn ManagedArtifact>>,
    kind: ArtifactKind,
    target: PathBuf,
    bytes: Vec<u8>,
) -> Result<(), CliError> {
    match std::fs::read(&target) {
        Ok(current) if current == bytes => Ok(()),
        Ok(_) => Err(configure_apply_error(
            CLI_CONFIGURE_UNMANAGED_COLLISION,
            "a managed configure target contains user-owned changes",
            format!(
                "`{}` differs from its reviewed generated representation",
                target.display()
            ),
            "Review the exportable patch; sc-lint will not overwrite the existing file.",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            artifacts.push(Box::new(BytesArtifact::new(kind, target, bytes)));
            Ok(())
        }
        Err(error) => Err(configure_apply_error(
            CLI_CONFIGURE_UNMANAGED_COLLISION,
            "a managed configure target could not be read",
            error.to_string(),
            "Check repository permissions and review the configure plan before retrying.",
        )),
    }
}

#[expect(
    clippy::result_large_err,
    reason = "Reserved recipe collisions require structured configure recovery."
)]
fn reject_reserved_recipes(source: &str) -> Result<(), CliError> {
    for name in ["setup", "lint", "test", "upgrade"] {
        if source
            .lines()
            .any(|line| line.trim_start().starts_with(&format!("{name}:")))
        {
            return Err(configure_apply_error(
                CLI_CONFIGURE_UNMANAGED_COLLISION,
                "the existing Justfile defines an sc-lint reserved recipe",
                format!("reserved recipe `{name}` would be shadowed by managed integration"),
                "Review the exportable patch; sc-lint will not overwrite or shadow your recipe.",
            ));
        }
    }
    Ok(())
}

pub(crate) fn configure_apply_error(
    code: &'static str,
    message: &str,
    cause: impl Into<String>,
    recovery: &str,
) -> CliError {
    CliError::config(message)
        .with_code(code)
        .with_cause(cause)
        .with_detail("pointer", Value::Null)
        .with_detail("recovery", serde_json::json!("review_plan"))
        .with_suggested_action(recovery)
        .with_documentation("sc-lint docs configuration")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn remove_plan(path: &str, reason: &str) -> Value {
        json!({
            "operations": [{
                "operation_id": "legacy-remove-test",
                "path": path,
                "kind": "propose_remove",
                "reason": reason,
            }]
        })
    }

    #[test]
    fn stale_removal_plan_is_rejected_before_any_artifact_is_created() {
        let reviewed = json!({
            "plan_id": "sha256:111111",
            "preconditions": [{
                "path": ".github/actions/setup-sc-lint/action.yml",
                "source_digest": "sha256:111111",
            }],
        });
        let fresh = json!({
            "plan_id": "sha256:111111",
            "preconditions": [{
                "path": ".github/actions/setup-sc-lint/action.yml",
                "source_digest": "sha256:222222",
            }],
        });

        let error = ensure_reviewed_plan_matches(&reviewed, &fresh).expect_err("stale plan");

        assert_eq!(error.code(), "CLI.CONFIGURE_STALE_PLAN");
    }

    #[test]
    fn apply_removes_only_an_exact_allowlisted_legacy_artifact() {
        let root = tempfile::tempdir().expect("root");
        let target = root.path().join(".github/actions/setup-sc-lint/action.yml");
        std::fs::create_dir_all(target.parent().expect("parent")).expect("parent directory");
        std::fs::write(&target, "legacy action\n").expect("legacy action");
        let plan = remove_plan(
            ".github/actions/setup-sc-lint/action.yml",
            crate::configure::legacy::EXACT_SC_COMPOSE_04_REASON,
        );
        let mut artifacts = Vec::new();

        add_reviewed_removals(root.path(), &plan, &mut artifacts).expect("allowlisted removal");
        crate::configure::apply::commit(artifacts).expect("commit removal");

        assert!(!target.exists());
    }

    #[test]
    fn apply_rejects_forged_or_near_match_removal_operations() {
        let root = tempfile::tempdir().expect("root");
        let plan = remove_plan(
            ".just/similarly-named-but-user-owned.py",
            crate::configure::legacy::EXACT_SC_COMPOSE_04_REASON,
        );
        let mut artifacts = Vec::new();

        let error = add_reviewed_removals(root.path(), &plan, &mut artifacts)
            .expect_err("near-match path is not removable");

        assert_eq!(error.code(), CLI_CONFIGURE_UNMANAGED_COLLISION);
        assert!(artifacts.is_empty());
    }
}
