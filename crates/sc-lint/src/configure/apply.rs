//! Ordered, reversible writes for the private configure artifact boundary.

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::CliError;
use crate::configure::artifact::ManagedArtifact;

struct StagedArtifact {
    target: PathBuf,
    staged: PathBuf,
    backup: PathBuf,
    existed: bool,
}

struct ReplaceError {
    error: io::Error,
    changed_current: bool,
}

/// Test-only failure points use the production transaction path to prove that
/// staging and replacement failures restore every already-touched artifact.
#[allow(
    dead_code,
    reason = "Production always supplies None; the variants are exercised by the transaction fault tests."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InjectedFailure {
    Stage(usize),
    Rename(usize),
    RenameAfterBackup(usize),
    PostCommit(usize),
}

/// Validate all artifacts before touching disk, stage beside each target, then
/// commit in plan order.  Any failed stage or rename restores every earlier
/// target from its same-directory backup.
#[expect(
    clippy::result_large_err,
    reason = "Configure apply errors carry the stable user recovery envelope."
)]
pub(crate) fn commit(artifacts: Vec<Box<dyn ManagedArtifact>>) -> Result<(), CliError> {
    commit_inner(artifacts, None)
}

/// Test-only seam proving a concrete extension artifact rolls back through the
/// same transaction as core artifacts; production has no injectable path.
#[cfg(test)]
#[expect(
    clippy::result_large_err,
    reason = "The test seam preserves the production transaction error type."
)]
pub(crate) fn commit_with_post_commit_failure_for_test(
    artifacts: Vec<Box<dyn ManagedArtifact>>,
    index: usize,
) -> Result<(), CliError> {
    commit_inner(artifacts, Some(InjectedFailure::PostCommit(index)))
}

#[expect(
    clippy::result_large_err,
    reason = "The test-only failure seam exercises the same structured transaction recovery path."
)]
fn commit_inner(
    artifacts: Vec<Box<dyn ManagedArtifact>>,
    injected_failure: Option<InjectedFailure>,
) -> Result<(), CliError> {
    for artifact in &artifacts {
        artifact.validate_staged()?;
    }

    let nonce = format!("{}.{}", std::process::id(), unique_suffix());
    let mut staged = Vec::with_capacity(artifacts.len());
    for (index, artifact) in artifacts.iter().enumerate() {
        let target = artifact.target().to_path_buf();
        let parent = target
            .parent()
            .ok_or_else(|| stage_error("artifact target has no parent directory"))?;
        fs::create_dir_all(parent)
            .map_err(|error| stage_io_error("create artifact directory", &target, error))?;
        let file_name = target
            .file_name()
            .ok_or_else(|| stage_error("artifact target has no filename"))?;
        let staged_path = parent.join(format!(
            ".{}.sc-lint-stage-{nonce}-{index}",
            file_name.to_string_lossy()
        ));
        let backup_path = parent.join(format!(
            ".{}.sc-lint-backup-{nonce}-{index}",
            file_name.to_string_lossy()
        ));
        if !artifact.is_removal() {
            if injected_failure == Some(InjectedFailure::Stage(index)) {
                cleanup_stages(&staged);
                return Err(stage_io_error(
                    "stage generated artifact",
                    &target,
                    io::Error::other("test-only injected stage failure"),
                ));
            }
            if let Err(error) = fs::write(&staged_path, artifact.staged_bytes()) {
                cleanup_stages(&staged);
                return Err(stage_io_error("stage generated artifact", &target, error));
            }
            if let Err(error) = preserve_mode(&target, &staged_path, artifact.kind()) {
                let _ = fs::remove_file(&staged_path);
                cleanup_stages(&staged);
                return Err(error);
            }
        }
        staged.push(StagedArtifact {
            existed: target.exists(),
            target,
            staged: staged_path,
            backup: backup_path,
        });
    }

    for index in 0..staged.len() {
        if injected_failure == Some(InjectedFailure::Rename(index)) {
            return rollback_error(
                &staged,
                index,
                index,
                io::Error::other("test-only injected rename failure"),
            );
        }
        if let Err(error) = replace_one(
            &staged[index],
            injected_failure == Some(InjectedFailure::RenameAfterBackup(index)),
        ) {
            return rollback_error(
                &staged,
                index,
                index + usize::from(error.changed_current),
                error.error,
            );
        }
        if injected_failure == Some(InjectedFailure::PostCommit(index)) {
            return rollback_error(
                &staged,
                index,
                index + 1,
                io::Error::other("test-only injected commit failure"),
            );
        }
    }
    for item in &staged {
        if item.existed {
            fs::remove_file(&item.backup)
                .map_err(|error| commit_io_error("remove committed backup", &item.backup, error))?;
        }
    }
    Ok(())
}

#[expect(
    clippy::result_large_err,
    reason = "Rollback reports the shared structured configure recovery envelope."
)]
fn rollback_error(
    staged: &[StagedArtifact],
    index: usize,
    changed_count: usize,
    error: io::Error,
) -> Result<(), CliError> {
    let rollback_error = rollback(&staged[..changed_count]);
    cleanup_stages(staged);
    Err(match rollback_error {
        Ok(()) => commit_io_error("commit generated artifact", &staged[index].target, error),
        Err(paths) => CliError::config("configure apply could not restore every changed file")
            .with_code("CLI.CONFIGURE_ROLLBACK_FAILED")
            .with_cause(error.to_string())
            .with_detail("backup_paths", serde_json::json!(paths))
            .with_suggested_action(
                "Restore the listed backup paths, then regenerate and review the configure plan.",
            )
            .with_documentation("sc-lint docs troubleshooting"),
    })
}

fn replace_one(
    item: &StagedArtifact,
    inject_failure_after_backup: bool,
) -> Result<(), ReplaceError> {
    if item.existed {
        fs::rename(&item.target, &item.backup).map_err(|error| ReplaceError {
            error,
            changed_current: false,
        })?;
        if inject_failure_after_backup {
            return Err(ReplaceError {
                error: io::Error::other("test-only injected post-backup rename failure"),
                changed_current: true,
            });
        }
    }
    if item.staged.exists() {
        fs::rename(&item.staged, &item.target).map_err(|error| ReplaceError {
            error,
            changed_current: item.existed,
        })
    } else {
        Ok(())
    }
}

fn rollback(items: &[StagedArtifact]) -> Result<(), Vec<String>> {
    let mut failed = Vec::new();
    for item in items.iter().rev() {
        if item.target.exists() && fs::remove_file(&item.target).is_err() {
            failed.push(item.target.display().to_string());
            continue;
        }
        if item.existed && fs::rename(&item.backup, &item.target).is_err() {
            failed.push(item.backup.display().to_string());
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(failed)
    }
}

fn cleanup_stages(items: &[StagedArtifact]) {
    for item in items {
        let _ = fs::remove_file(&item.staged);
    }
}

#[cfg(unix)]
#[expect(
    clippy::result_large_err,
    reason = "Permission preservation participates in the shared structured configure error path."
)]
fn preserve_mode(
    target: &std::path::Path,
    staged: &std::path::Path,
    kind: crate::configure::artifact::ArtifactKind,
) -> Result<(), CliError> {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = fs::metadata(target) {
        let permissions = fs::Permissions::from_mode(metadata.permissions().mode());
        fs::set_permissions(staged, permissions)
            .map_err(|error| stage_io_error("preserve artifact mode", target, error))?;
    } else if kind == crate::configure::artifact::ArtifactKind::Shell {
        fs::set_permissions(staged, fs::Permissions::from_mode(0o755)).map_err(|error| {
            stage_io_error("make generated shell helper executable", target, error)
        })?;
    }
    Ok(())
}

#[cfg(windows)]
#[expect(
    clippy::result_large_err,
    reason = "Windows has no POSIX mode to preserve, but shares the configure error signature."
)]
fn preserve_mode(
    _target: &std::path::Path,
    _staged: &std::path::Path,
    _kind: crate::configure::artifact::ArtifactKind,
) -> Result<(), CliError> {
    // Windows does not expose the POSIX executable-mode contract preserved by
    // the Unix implementation, so staging has no permission mutation to make.
    Ok(())
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn stage_error(message: &str) -> CliError {
    CliError::config(message)
        .with_code("CLI.CONFIGURE_STAGE_FAILED")
        .with_suggested_action("Regenerate and review the configure plan, then retry.")
        .with_documentation("sc-lint docs troubleshooting")
}

fn stage_io_error(operation: &str, path: &std::path::Path, error: io::Error) -> CliError {
    CliError::config(format!("failed to {operation} `{}`", path.display()))
        .with_code("CLI.CONFIGURE_STAGE_FAILED")
        .with_source(error)
        .with_suggested_action(
            "Check repository permissions, then regenerate and review the configure plan.",
        )
        .with_documentation("sc-lint docs troubleshooting")
}

fn commit_io_error(operation: &str, path: &std::path::Path, error: io::Error) -> CliError {
    CliError::config(format!("failed to {operation} `{}`", path.display()))
        .with_code("CLI.CONFIGURE_COMMIT_FAILED")
        .with_source(error)
        .with_suggested_action(
            "Check repository permissions, then regenerate and review the configure plan.",
        )
        .with_documentation("sc-lint docs troubleshooting")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configure::artifact::{ArtifactKind, BytesArtifact, RemoveArtifact};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    struct SyntheticArtifact {
        target: PathBuf,
        bytes: Vec<u8>,
        valid: bool,
    }
    impl ManagedArtifact for SyntheticArtifact {
        fn kind(&self) -> ArtifactKind {
            ArtifactKind::Json
        }
        fn target(&self) -> &Path {
            &self.target
        }
        fn staged_bytes(&self) -> &[u8] {
            &self.bytes
        }
        fn validate_staged(&self) -> Result<(), CliError> {
            if self.valid {
                Ok(())
            } else {
                Err(CliError::config("synthetic validation failure")
                    .with_code("CLI.CONFIGURE_UNMANAGED_COLLISION"))
            }
        }
    }

    #[test]
    fn invalid_extension_artifact_changes_no_prior_file() {
        let root = tempfile::tempdir().expect("temp root");
        let config = root.path().join("sc-lint.toml");
        fs::write(&config, "before = true\n").expect("initial config");
        let result = commit(vec![
            Box::new(BytesArtifact::new(
                ArtifactKind::Toml,
                config.clone(),
                b"after = true\n".to_vec(),
            )),
            Box::new(SyntheticArtifact {
                target: root.path().join("synthetic.json"),
                bytes: b"{}".to_vec(),
                valid: false,
            }),
        ]);
        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(config).expect("config remains"),
            "before = true\n"
        );
    }

    #[test]
    fn preserves_existing_mode_when_replacing() {
        let root = tempfile::tempdir().expect("temp root");
        let config = root.path().join("sc-lint.toml");
        fs::write(&config, "before = true\n").expect("initial config");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config, fs::Permissions::from_mode(0o640)).expect("mode");
        }
        commit(vec![Box::new(BytesArtifact::new(
            ArtifactKind::Toml,
            config.clone(),
            b"after = true\n".to_vec(),
        ))])
        .expect("commit");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&config)
                    .expect("metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o640
            );
        }
    }

    #[test]
    fn removal_is_committed_through_the_same_backup_transaction() {
        let root = tempfile::tempdir().expect("temp root");
        let target = root.path().join("legacy.json");
        fs::write(&target, "{}\n").expect("legacy target");
        commit(vec![Box::new(RemoveArtifact::new(
            ArtifactKind::Json,
            target.clone(),
        ))])
        .expect("remove");
        assert!(!target.exists());
    }

    #[test]
    fn removal_is_restored_when_a_later_transaction_step_fails() {
        let root = tempfile::tempdir().expect("temp root");
        let legacy = root.path().join("legacy.py");
        let generated = root.path().join("generated.toml");
        fs::write(&legacy, "legacy bytes\n").expect("legacy target");
        fs::write(&generated, "value = 'before'\n").expect("generated target");
        let result = commit_inner(
            vec![
                Box::new(RemoveArtifact::new(ArtifactKind::Shell, legacy.clone())),
                Box::new(BytesArtifact::new(
                    ArtifactKind::Toml,
                    generated.clone(),
                    b"value = 'after'\n".to_vec(),
                )),
            ],
            Some(InjectedFailure::PostCommit(1)),
        );
        assert_eq!(
            result.expect_err("commit failure").code(),
            "CLI.CONFIGURE_COMMIT_FAILED"
        );
        assert_eq!(
            fs::read_to_string(legacy).expect("legacy restored"),
            "legacy bytes\n"
        );
        assert_eq!(
            fs::read_to_string(generated).expect("generated restored"),
            "value = 'before'\n"
        );
    }

    #[test]
    fn malformed_json_is_rejected_before_any_write() {
        let root = tempfile::tempdir().expect("temp root");
        let target = root.path().join("generated.json");
        let result = commit(vec![Box::new(BytesArtifact::new(
            ArtifactKind::Json,
            target.clone(),
            b"not json".to_vec(),
        ))]);
        assert!(result.is_err());
        assert!(!target.exists());
    }

    #[test]
    fn malformed_toml_and_just_are_rejected_before_any_write() {
        let root = tempfile::tempdir().expect("temp root");
        let existing = root.path().join("existing.toml");
        fs::write(&existing, "value = 'before'\n").expect("existing bytes");
        let result = commit(vec![
            Box::new(BytesArtifact::new(
                ArtifactKind::Toml,
                existing.clone(),
                b"value = [\n".to_vec(),
            )),
            Box::new(BytesArtifact::new(
                ArtifactKind::Justfile,
                root.path().join("Justfile"),
                b"# >>> sc-lint managed integration >>>\n".to_vec(),
            )),
        ]);
        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(existing).expect("existing remains"),
            "value = 'before'\n"
        );
        assert!(!root.path().join("Justfile").exists());
    }

    #[test]
    fn injected_stage_failure_leaves_all_original_bytes_and_modes() {
        let root = tempfile::tempdir().expect("temp root");
        let first = root.path().join("first.toml");
        fs::write(&first, "value = 'before'\n").expect("first before");
        #[cfg(unix)]
        fs::set_permissions(&first, fs::Permissions::from_mode(0o640)).expect("first mode");
        let result = commit_inner(
            vec![
                Box::new(BytesArtifact::new(
                    ArtifactKind::Toml,
                    first.clone(),
                    b"value = 'after'\n".to_vec(),
                )),
                Box::new(BytesArtifact::new(
                    ArtifactKind::Json,
                    root.path().join("second.json"),
                    b"{}\n".to_vec(),
                )),
            ],
            Some(InjectedFailure::Stage(1)),
        );
        assert_eq!(
            result.expect_err("stage failure").code(),
            "CLI.CONFIGURE_STAGE_FAILED"
        );
        assert_eq!(
            fs::read_to_string(&first).expect("first remains"),
            "value = 'before'\n"
        );
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&first).expect("metadata").permissions().mode() & 0o777,
            0o640
        );
        assert!(!root.path().join("second.json").exists());
    }

    #[test]
    fn injected_synthetic_rename_failure_restores_toml_just_and_mode() {
        let root = tempfile::tempdir().expect("temp root");
        let first = root.path().join("first.toml");
        let justfile = root.path().join("Justfile");
        let synthetic = root.path().join("synthetic.extension");
        fs::write(&first, "value = 'before-first'\n").expect("first before");
        fs::write(&justfile, "user:\n    @echo before\n").expect("Justfile before");
        fs::write(&synthetic, "extension before\n").expect("synthetic before");
        #[cfg(unix)]
        fs::set_permissions(&first, fs::Permissions::from_mode(0o640)).expect("first mode");
        let result = commit_inner(
            vec![
                Box::new(BytesArtifact::new(
                    ArtifactKind::Toml,
                    first.clone(),
                    b"value = 'after-first'\n".to_vec(),
                )),
                Box::new(BytesArtifact::new(
                    ArtifactKind::Justfile,
                    justfile.clone(),
                    b"user:\n    @echo after\n".to_vec(),
                )),
                Box::new(SyntheticArtifact {
                    target: synthetic.clone(),
                    bytes: b"extension output".to_vec(),
                    valid: true,
                }),
            ],
            Some(InjectedFailure::RenameAfterBackup(2)),
        );
        assert_eq!(
            result.expect_err("commit failure").code(),
            "CLI.CONFIGURE_COMMIT_FAILED"
        );
        assert_eq!(
            fs::read_to_string(&first).expect("first restored"),
            "value = 'before-first'\n"
        );
        assert_eq!(
            fs::read_to_string(justfile).expect("Justfile restored"),
            "user:\n    @echo before\n"
        );
        assert_eq!(
            fs::read_to_string(synthetic).expect("synthetic restored"),
            "extension before\n"
        );
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&first).expect("metadata").permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn injected_post_commit_failure_restores_every_prior_artifact() {
        let root = tempfile::tempdir().expect("temp root");
        let first = root.path().join("first.toml");
        let second = root.path().join("second.json");
        fs::write(&first, "value = 'before-first'\n").expect("first before");
        fs::write(&second, "{\"value\": \"before\"}\n").expect("second before");
        let result = commit_inner(
            vec![
                Box::new(BytesArtifact::new(
                    ArtifactKind::Toml,
                    first.clone(),
                    b"value = 'after-first'\n".to_vec(),
                )),
                Box::new(BytesArtifact::new(
                    ArtifactKind::Json,
                    second.clone(),
                    b"{\"value\": \"after\"}\n".to_vec(),
                )),
            ],
            Some(InjectedFailure::PostCommit(1)),
        );
        assert_eq!(
            result.expect_err("commit failure").code(),
            "CLI.CONFIGURE_COMMIT_FAILED"
        );
        assert_eq!(
            fs::read_to_string(first).expect("first restored"),
            "value = 'before-first'\n"
        );
        assert_eq!(
            fs::read_to_string(second).expect("second restored"),
            "{\"value\": \"before\"}\n"
        );
    }
}
