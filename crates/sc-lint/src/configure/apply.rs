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

/// Validate all artifacts before touching disk, stage beside each target, then
/// commit in plan order.  Any failed stage or rename restores every earlier
/// target from its same-directory backup.
#[expect(
    clippy::result_large_err,
    reason = "Configure apply errors carry the stable user recovery envelope."
)]
pub(crate) fn commit(artifacts: Vec<Box<dyn ManagedArtifact>>) -> Result<(), CliError> {
    for artifact in &artifacts {
        artifact.validate_staged()?;
    }

    let nonce = format!("{}.{}", std::process::id(), unique_suffix());
    let mut staged = Vec::with_capacity(artifacts.len());
    for (index, artifact) in artifacts.iter().enumerate() {
        let target = artifact.target().to_path_buf();
        let parent = target
            .parent()
            .ok_or_else(|| transaction_error("artifact target has no parent directory"))?;
        fs::create_dir_all(parent)
            .map_err(|error| io_error("create artifact directory", &target, error))?;
        let file_name = target
            .file_name()
            .ok_or_else(|| transaction_error("artifact target has no filename"))?;
        let staged_path = parent.join(format!(
            ".{}.sc-lint-stage-{nonce}-{index}",
            file_name.to_string_lossy()
        ));
        let backup_path = parent.join(format!(
            ".{}.sc-lint-backup-{nonce}-{index}",
            file_name.to_string_lossy()
        ));
        if let Err(error) = fs::write(&staged_path, artifact.staged_bytes()) {
            cleanup_stages(&staged);
            return Err(io_error("stage generated artifact", &target, error));
        }
        preserve_mode(&target, &staged_path, artifact.kind())?;
        staged.push(StagedArtifact {
            existed: target.exists(),
            target,
            staged: staged_path,
            backup: backup_path,
        });
    }

    for index in 0..staged.len() {
        if let Err(error) = replace_one(&staged[index]) {
            let rollback_error = rollback(&staged[..=index]);
            cleanup_stages(&staged);
            return Err(match rollback_error {
                Ok(()) => io_error("commit generated artifact", &staged[index].target, error),
                Err(paths) => CliError::config("configure apply could not restore every changed file")
                    .with_code("CLI.CONFIGURE_ROLLBACK_FAILED")
                    .with_cause(error.to_string())
                    .with_detail("backup_paths", serde_json::json!(paths))
                    .with_suggested_action("Restore the listed backup paths, then regenerate and review the configure plan.")
                    .with_documentation("sc-lint docs troubleshooting"),
            });
        }
    }
    for item in &staged {
        if item.existed {
            fs::remove_file(&item.backup)
                .map_err(|error| io_error("remove committed backup", &item.backup, error))?;
        }
    }
    Ok(())
}

fn replace_one(item: &StagedArtifact) -> io::Result<()> {
    if item.existed {
        fs::rename(&item.target, &item.backup)?;
    }
    fs::rename(&item.staged, &item.target)
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
            .map_err(|error| io_error("preserve artifact mode", target, error))?;
    } else if kind == crate::configure::artifact::ArtifactKind::Shell {
        fs::set_permissions(staged, fs::Permissions::from_mode(0o755))
            .map_err(|error| io_error("make generated shell helper executable", target, error))?;
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
    Ok(())
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn transaction_error(message: &str) -> CliError {
    CliError::config(message)
        .with_code("CLI.CONFIGURE_ROLLBACK_FAILED")
        .with_suggested_action("Regenerate and review the configure plan, then retry.")
        .with_documentation("sc-lint docs troubleshooting")
}

fn io_error(operation: &str, path: &std::path::Path, error: io::Error) -> CliError {
    CliError::config(format!("failed to {operation} `{}`", path.display()))
        .with_code("CLI.CONFIGURE_ROLLBACK_FAILED")
        .with_source(error)
        .with_suggested_action(
            "Check repository permissions, then regenerate and review the configure plan.",
        )
        .with_documentation("sc-lint docs troubleshooting")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configure::artifact::{ArtifactKind, BytesArtifact};
    use std::path::Path;

    struct SyntheticArtifact {
        target: PathBuf,
        bytes: Vec<u8>,
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
            Err(CliError::config("synthetic validation failure")
                .with_code("CLI.CONFIGURE_UNMANAGED_COLLISION"))
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
}
