use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::command::ConsumerInitRequest;
use crate::config::CONFIG_FILENAME;
use crate::installer::CONSUMER_BOOTSTRAP_ASSET;

const GENERATED_FILE_HEADER: &str =
    "# Managed by sc-lint; regenerate with `sc-lint init --just`.\n";
const CONSUMER_JUSTFILE_ASSET: &str = include_str!("../assets/consumer-Justfile");
const CONSUMER_CONFIG_ASSET: &str = include_str!("../assets/consumer-config.toml");
const CONFIG_VERSION_TOKEN: &str = "{{SC_LINT_VERSION}}";
pub(crate) const BINARY_NOT_FOUND_RECOVERY: &str = "Run `just setup` (or your repository's sc-lint installer) to create or repair a compatible installation.";
pub(crate) const DOCS_SETUP_REFERENCE: &str = "sc-lint docs setup";

/// The classified ownership state of one product-managed consumer file.
/// Generated configurations with only a version change are safe to refresh;
/// all other drift stays user-owned and is never overwritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedFileState {
    Current,
    Missing,
    RefreshableGenerated,
    UserModified,
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer initialization preserves the shared top-level CliError contract."
)]
pub(crate) fn run_consumer_init(request: ConsumerInitRequest) -> Result<Value, CliError> {
    let root = std::env::current_dir().map_err(|error| {
        CliError::config("failed to read current directory for consumer initialization")
            .with_source(error)
            .with_suggested_action(
                "Change to the consumer repository and rerun `sc-lint init --just`.",
            )
            .with_documentation(DOCS_SETUP_REFERENCE)
    })?;
    run_consumer_init_at(&root, request)
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer integration file ownership errors use the shared top-level CliError contract."
)]
pub(crate) fn run_consumer_init_at(
    root: &Path,
    request: ConsumerInitRequest,
) -> Result<Value, CliError> {
    if !request.just {
        return Err(CliError::usage("`sc-lint init` requires `--just`")
            .with_suggested_action(
                "Run `sc-lint init --just` to create the canonical consumer integration.",
            )
            .with_documentation(DOCS_SETUP_REFERENCE));
    }
    if request.check && request.dry_run {
        return Err(
            CliError::usage("`--check` and `--dry-run` cannot be combined")
                .with_suggested_action(
                    "Use either `--check` or `--dry-run`, then rerun `sc-lint init --just`.",
                )
                .with_documentation(DOCS_SETUP_REFERENCE),
        );
    }

    let files = consumer_integration_files(root);
    let mut current = Vec::new();
    let mut missing = Vec::new();
    let mut refreshable = Vec::new();
    let mut conflicts = Vec::new();
    for file in &files {
        match classify_file(file)? {
            ManagedFileState::Current => current.push(file.path.clone()),
            ManagedFileState::Missing => missing.push(file.path.clone()),
            ManagedFileState::RefreshableGenerated => refreshable.push(file.path.clone()),
            ManagedFileState::UserModified => conflicts.push(file.path.clone()),
        }
    }
    let write_needed = missing
        .iter()
        .chain(&refreshable)
        .cloned()
        .collect::<Vec<_>>();
    if !conflicts.is_empty() {
        return Err(CliError::config("consumer integration contains user-owned file conflicts")
            .with_code("CLI.SC_LINT_INTEGRATION_CONFLICT")
            .with_detail("conflicts", paths_to_json(&conflicts))
            .with_suggested_action(
                "Move or remove the conflicting file, then rerun `sc-lint init --just`; sc-lint will not overwrite it.",
            )
            .with_documentation(DOCS_SETUP_REFERENCE));
    }
    if request.check && !write_needed.is_empty() {
        return Err(CliError::config("consumer integration is not current")
            .with_code("CLI.SC_LINT_INTEGRATION_OUTDATED")
            .with_detail("outdated", paths_to_json(&write_needed))
            .with_suggested_action(
                "Run `sc-lint init --just` to create or refresh the managed files.",
            )
            .with_documentation(DOCS_SETUP_REFERENCE));
    }
    if !request.check && !request.dry_run {
        for file in &files {
            if write_needed.contains(&file.path) {
                if let Some(parent) = file.path.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        CliError::config(format!(
                            "failed to create consumer integration directory `{}`",
                            parent.display()
                        ))
                        .with_source(error)
                        .with_suggested_action(
                            "Check directory permissions, then rerun `sc-lint init --just`.",
                        )
                        .with_documentation(DOCS_SETUP_REFERENCE)
                    })?;
                }
                fs::write(&file.path, &file.contents).map_err(|error| {
                    CliError::config(format!(
                        "failed to write consumer integration file `{}`",
                        file.path.display()
                    ))
                    .with_source(error)
                    .with_suggested_action(
                        "Check file permissions, then rerun `sc-lint init --just`.",
                    )
                    .with_documentation(DOCS_SETUP_REFERENCE)
                })?;
                set_bootstrap_permissions(&file.path)?;
            }
        }
    }

    Ok(json!({
        "status": if write_needed.is_empty() { "current" } else if request.dry_run && refreshable.is_empty() { "would_create" } else if request.dry_run { "would_update" } else if refreshable.is_empty() { "created" } else { "updated" },
        "managed_files": files.iter().map(|file| file.path.display().to_string()).collect::<Vec<_>>(),
        "current_files": paths_to_strings(&current),
        "created_or_updated_files": paths_to_strings(&write_needed),
        "check": request.check,
        "dry_run": request.dry_run,
        "summary": "consumer Just integration is managed by sc-lint",
    }))
}

struct ConsumerIntegrationFile {
    path: PathBuf,
    contents: String,
    kind: ManagedFileKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedFileKind {
    Config,
    Static,
}

fn consumer_integration_files(root: &Path) -> Vec<ConsumerIntegrationFile> {
    vec![
        ConsumerIntegrationFile {
            path: root.join(CONFIG_FILENAME),
            contents: canonical_consumer_config(),
            kind: ManagedFileKind::Config,
        },
        ConsumerIntegrationFile {
            path: root.join("Justfile"),
            contents: format!("{GENERATED_FILE_HEADER}{CONSUMER_JUSTFILE_ASSET}"),
            kind: ManagedFileKind::Static,
        },
        ConsumerIntegrationFile {
            path: root.join(".sc-lint/bootstrap"),
            contents: generated_bootstrap_asset(),
            kind: ManagedFileKind::Static,
        },
    ]
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer file inspection returns the shared top-level CliError contract."
)]
fn classify_file(file: &ConsumerIntegrationFile) -> Result<ManagedFileState, CliError> {
    match fs::read_to_string(&file.path) {
        Ok(actual) if actual == file.contents => Ok(ManagedFileState::Current),
        Ok(actual)
            if file.kind == ManagedFileKind::Config && is_refreshable_generated_config(&actual) =>
        {
            Ok(ManagedFileState::RefreshableGenerated)
        }
        Ok(_) => Ok(ManagedFileState::UserModified),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ManagedFileState::Missing),
        Err(error) => Err(CliError::config(format!(
            "failed to inspect consumer integration file `{}`",
            file.path.display()
        ))
        .with_source(error)
        .with_suggested_action("Check file permissions, then rerun `sc-lint init --just`.")
        .with_documentation(DOCS_SETUP_REFERENCE)),
    }
}

fn is_refreshable_generated_config(actual: &str) -> bool {
    let (prefix, suffix) = CONSUMER_CONFIG_ASSET
        .split_once(CONFIG_VERSION_TOKEN)
        .expect("consumer config asset must contain its version token");
    let body = actual
        .strip_prefix(GENERATED_FILE_HEADER)
        .unwrap_or_default();
    body.starts_with(prefix) && body.ends_with(suffix) && body.len() > prefix.len() + suffix.len()
}

fn generated_bootstrap_asset() -> String {
    const SHEBANG: &str = "#!/bin/sh\n";
    let asset = CONSUMER_BOOTSTRAP_ASSET
        .strip_prefix(SHEBANG)
        .expect("consumer bootstrap asset must start with a POSIX shell shebang");
    format!("{SHEBANG}{GENERATED_FILE_HEADER}{asset}")
}

fn canonical_consumer_config() -> String {
    format!(
        "{GENERATED_FILE_HEADER}{}",
        CONSUMER_CONFIG_ASSET.replace(CONFIG_VERSION_TOKEN, env!("CARGO_PKG_VERSION"))
    )
}

fn paths_to_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

fn paths_to_json(paths: &[PathBuf]) -> Value {
    json!(paths_to_strings(paths))
}

#[expect(
    clippy::result_large_err,
    reason = "Generated bootstrap permission errors use the shared top-level CliError contract."
)]
fn set_bootstrap_permissions(path: &Path) -> Result<(), CliError> {
    if path.file_name().is_some_and(|name| name == "bootstrap") {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(path)
                .map_err(|error| {
                    CliError::config("failed to inspect generated bootstrap").with_source(error)
                })?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).map_err(|error| {
                CliError::config("failed to make generated bootstrap executable")
                    .with_source(error)
                    .with_suggested_action(
                        "Check file permissions, then rerun `sc-lint init --just`.",
                    )
                    .with_documentation(DOCS_SETUP_REFERENCE)
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CompatibilityErrorCode;
    use crate::error::ErrorCode;

    #[test]
    fn bootstrap_missing_binary_contract_tracks_the_structured_error_contract() {
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains(CompatibilityErrorCode::BinaryNotFound.as_str()));
        assert!(
            CONSUMER_BOOTSTRAP_ASSET
                .replace("\\`", "`")
                .contains(BINARY_NOT_FOUND_RECOVERY)
        );
        assert!(CONSUMER_BOOTSTRAP_ASSET.contains(DOCS_SETUP_REFERENCE));
    }

    #[test]
    fn generated_config_with_only_a_version_difference_is_refreshable() {
        let old = canonical_consumer_config().replace(env!("CARGO_PKG_VERSION"), "0.0.0");
        assert!(is_refreshable_generated_config(&old));
        assert!(!is_refreshable_generated_config(
            "[tool.sc-lint]\nminimum_version = \"0.0.0\"\n"
        ));
    }

    #[test]
    fn version_bumped_generated_config_is_refreshed_not_reported_as_user_conflict() {
        let fixture = tempfile::TempDir::new().expect("fixture");
        let request = ConsumerInitRequest {
            just: true,
            check: false,
            dry_run: false,
        };
        run_consumer_init_at(fixture.path(), request).expect("initial integration");
        let config_path = fixture.path().join(CONFIG_FILENAME);
        let old = fs::read_to_string(&config_path)
            .expect("generated config")
            .replace(env!("CARGO_PKG_VERSION"), "0.0.0");
        fs::write(&config_path, old).expect("old generated config");

        let result =
            run_consumer_init_at(fixture.path(), request).expect("refresh generated config");
        assert_eq!(result["status"], "updated");
        assert_eq!(
            fs::read_to_string(config_path).expect("refreshed config"),
            canonical_consumer_config()
        );
    }
}
