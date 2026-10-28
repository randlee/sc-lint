//! Payload activation with backup and rollback for managed installs.

use super::*;

pub(super) trait InstallerFileOps {
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()>;

    fn remove_path(&self, path: &Path) -> io::Result<()> {
        if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            self.remove_file(path)
        }
    }
}

pub(super) struct SystemFileOps;

impl InstallerFileOps for SystemFileOps {
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()> {
        fs::rename(source, destination)
    }

    fn remove_path(&self, path: &Path) -> io::Result<()> {
        if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }
}

pub(super) fn activate_payload(
    entries: &[PayloadEntry],
    managed_binary: &Path,
    minimum: &MinimumVersion,
) -> Result<Version, CliError> {
    activate_payload_with(entries, managed_binary, minimum, &SystemFileOps)
}

pub(super) fn activate_payload_with<F: InstallerFileOps>(
    entries: &[PayloadEntry],
    managed_binary: &Path,
    minimum: &MinimumVersion,
    file_ops: &F,
) -> Result<Version, CliError> {
    reject_running_managed_binary(managed_binary)?;
    let backups = prepare_payload_activation(entries, managed_binary)?;
    let changes = activate_payload_entries(entries, &backups, managed_binary, file_ops)?;
    match verify_activated_payload(managed_binary, minimum) {
        Ok(version) => {
            remove_payload_backups(&changes, file_ops);
            Ok(version)
        }
        Err(cause) => {
            if let Err(rollback_error) = rollback_payload(&changes, file_ops) {
                let backup = changes
                    .first()
                    .map(|(_, backup, _, _)| backup.as_path())
                    .unwrap_or(managed_binary);
                return Err(
                    rollback_failure_error(managed_binary, backup, rollback_error, None)
                        .with_detail("post_install_verification_cause", json!(cause)),
                );
            }
            Err(install_error(
                InstallerErrorCode::PostInstallVersionFailed,
                format!("activated sc-lint bundle `{}` failed post-install verification", managed_binary.display()),
                cause,
                "The previous managed bundle was restored. Repair the release source and rerun `sc-lint setup`.",
            )
            .with_detail("managed_binary", json!(managed_binary.display().to_string()))
            .with_detail("minimum_version", json!(minimum.to_string())))
        }
    }
}

pub(super) fn prepare_payload_activation(
    entries: &[PayloadEntry],
    managed_binary: &Path,
) -> Result<Vec<PathBuf>, CliError> {
    let mut backups = Vec::with_capacity(entries.len());
    for entry in entries {
        let parent = entry.target.parent().ok_or_else(|| {
            activation_error(
                &entry.target,
                io::Error::other("target has no parent directory"),
            )
        })?;
        fs::create_dir_all(parent).map_err(|error| permission_error(parent, error))?;
        let file_name = entry
            .target
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let backup = parent.join(format!(".{file_name}-sc-lint-backup"));
        if backup.exists() {
            return Err(install_error(
                InstallerErrorCode::RollbackFailed,
                format!("managed payload backup `{}` already exists", backup.display()),
                "A prior activation left a backup that may contain the previous managed payload.",
                "Inspect the backup and managed installation, restore the known-good payload if needed, then rerun `sc-lint setup`.",
            )
            .with_detail("managed_binary", json!(managed_binary.display().to_string()))
            .with_detail("backup_path", json!(backup.display().to_string())));
        }
        backups.push(backup);
    }
    Ok(backups)
}

pub(super) fn activate_payload_entries<F: InstallerFileOps>(
    entries: &[PayloadEntry],
    backups: &[PathBuf],
    managed_binary: &Path,
    file_ops: &F,
) -> Result<Vec<PayloadChange>, CliError> {
    let mut changes = Vec::with_capacity(entries.len());
    for (entry, backup) in entries.iter().zip(backups) {
        let had_previous = entry.target.exists();
        if had_previous && let Err(error) = file_ops.rename(&entry.target, backup) {
            let error_message = error.to_string();
            rollback_payload(&changes, file_ops).map_err(|rollback_error| {
                rollback_failure_error(
                    managed_binary,
                    backup,
                    rollback_error,
                    Some(io::Error::other(error_message.clone())),
                )
            })?;
            return Err(activation_error(
                &entry.target,
                io::Error::other(error_message),
            ));
        }
        changes.push((entry.target.clone(), backup.clone(), had_previous, false));
        if let Err(error) = file_ops.rename(&entry.source, &entry.target) {
            if let Err(rollback_error) = rollback_payload(&changes, file_ops) {
                return Err(rollback_failure_error(
                    managed_binary,
                    backup,
                    rollback_error,
                    Some(error),
                ));
            }
            return Err(activation_error(&entry.target, error));
        }
        if let Some(last) = changes.last_mut() {
            last.3 = true;
        }
    }
    Ok(changes)
}

pub(super) fn verify_activated_payload(
    managed_binary: &Path,
    minimum: &MinimumVersion,
) -> Result<Version, String> {
    let version = probe_version(managed_binary)
        .map_err(|failure| failure.into_message(managed_binary, crate::consts::SERVICE_NAME))?;
    if version < *minimum.as_semver() {
        return Err(format!(
            "activated version `{version}` is below required `{minimum}`"
        ));
    }
    if let Some(problem) = managed_payload_validation_issue(managed_binary) {
        return Err(format!(
            "activated release payload failed validation: {problem}"
        ));
    }
    Ok(version)
}

pub(super) fn remove_payload_backups<F: InstallerFileOps>(changes: &[PayloadChange], file_ops: &F) {
    for (_, backup, had_previous, _) in changes {
        if *had_previous {
            let _ = file_ops.remove_path(backup);
        }
    }
}

pub(super) fn rollback_payload<F: InstallerFileOps>(
    changes: &[PayloadChange],
    file_ops: &F,
) -> io::Result<()> {
    for (target, backup, had_previous, activated) in changes.iter().rev() {
        if *activated && target.exists() {
            file_ops.remove_path(target)?;
        }
        if *had_previous && backup.exists() {
            file_ops.rename(backup, target)?;
        }
    }
    Ok(())
}

pub(super) fn rollback_failure_error(
    target: &Path,
    backup: &Path,
    rollback_error: io::Error,
    activation_error: Option<io::Error>,
) -> CliError {
    let mut error = install_error(
        InstallerErrorCode::RollbackFailed,
        format!(
            "could not verify rollback of managed sc-lint binary `{}`",
            target.display()
        ),
        rollback_error.to_string(),
        format!(
            "Inspect `{}` and `{}`; recover the known-good backup manually, then rerun `sc-lint setup`.",
            target.display(),
            backup.display()
        ),
    )
    .with_detail("managed_binary", json!(target.display().to_string()))
    .with_detail("backup_path", json!(backup.display().to_string()))
    .with_detail(
        "rollback_error_kind",
        json!(format!("{:?}", rollback_error.kind())),
    );
    if let Some(activation_error) = activation_error {
        error = error.with_detail("activation_error", json!(activation_error.to_string()));
    }
    error
}
