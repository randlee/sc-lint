use std::error::Error;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::analyze_workspace;
use sc_lint_schema::OutputFormat;
use serde_json::Error as JsonError;
use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::command::CommandContext;
use crate::command::CommandSuccess;
use crate::command::DispatchTelemetry;
use crate::config::LoadedConfig;
use crate::consts;

#[derive(Debug)]
enum BoundaryDispatchError {
    Analysis(Box<dyn Error + Send + Sync>),
    Serialize(JsonError),
    Normalize(JsonError),
}

impl BoundaryDispatchError {
    fn into_cli_error(self, tool: &str, repo_root: Option<&Path>) -> CliError {
        match self {
            Self::Analysis(error) => {
                CliError::backend_failure(format!("{tool} failed to analyze the workspace"))
                    .with_source(error)
                    .with_detail(consts::FIELD_TOOL, json!(tool))
                    .with_detail(
                        consts::FIELD_ROOT,
                        json!(repo_root.map(|root| root.display().to_string())),
                    )
                    .with_suggested_action(
                        "Check the boundary inventory and workspace sources, then rerun `sc-lint lint sc-boundary` for a focused failure report.",
                    )
            }
            Self::Serialize(error) => CliError::backend_protocol(format!(
                "{tool} produced a report that could not be encoded as machine JSON"
            ))
            .with_source(error)
            .with_detail(consts::FIELD_TOOL, json!(tool))
            .with_suggested_action(
                "Inspect the backend report payload for non-serializable fields and rerun the analyzer.",
            ),
            Self::Normalize(error) => {
                CliError::backend_protocol(format!("{tool} returned malformed machine output"))
                    .with_source(error)
                    .with_detail(consts::FIELD_TOOL, json!(tool))
                    .with_suggested_action(
                        "Run the backend directly and inspect its JSON output for schema mismatches.",
                    )
            }
        }
    }
}

#[expect(
    clippy::result_large_err,
    reason = "Dispatch failures must remain in the shared top-level CliError contract."
)]
pub fn run_sc_boundary(
    _context: &CommandContext,
    loaded_config: &LoadedConfig,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let tool = consts::TOOL_BOUNDARY;
    let report = analyze_workspace(&AnalyzeOptions {
        root: repo_root.to_path_buf(),
        format: OutputFormat::Json,
        rule: None,
    })
    .map_err(|error| BoundaryDispatchError::Analysis(Box::new(error)))
    .map_err(|error| error.into_cli_error(tool, Some(repo_root)))?;

    let raw = serde_json::to_string(&report)
        .map_err(BoundaryDispatchError::Serialize)
        .map_err(|error| error.into_cli_error(tool, Some(repo_root)))?;
    let normalized = normalize_backend_json(tool, &raw)?;
    let finding_count = normalized
        .get(consts::FIELD_FINDINGS)
        .and_then(Value::as_array)
        .map_or(0, std::vec::Vec::len);

    Ok(CommandSuccess::with_dispatch(
        normalized,
        DispatchTelemetry::new(tool, finding_count),
    ))
}

#[expect(
    clippy::result_large_err,
    reason = "Delegated backend failures must remain in the shared top-level CliError contract."
)]
pub fn run_sc_portability(
    _context: &CommandContext,
    loaded_config: &LoadedConfig,
) -> Result<CommandSuccess, CliError> {
    run_delegated_backend(loaded_config, consts::TOOL_PORTABILITY)
}

#[expect(
    clippy::result_large_err,
    reason = "Delegated backend failures must remain in the shared top-level CliError contract."
)]
pub fn run_sc_runtime(
    _context: &CommandContext,
    loaded_config: &LoadedConfig,
) -> Result<CommandSuccess, CliError> {
    run_delegated_backend(loaded_config, consts::TOOL_RUNTIME)
}

#[expect(
    clippy::result_large_err,
    reason = "Backend protocol violations must remain in the shared top-level CliError contract."
)]
pub fn normalize_backend_json(tool: &str, raw: &str) -> Result<Value, CliError> {
    serde_json::from_str(raw)
        .map_err(BoundaryDispatchError::Normalize)
        .map_err(|error| error.into_cli_error(tool, None))
}

#[expect(
    clippy::result_large_err,
    reason = "Delegated backend failures must remain in the shared top-level CliError contract."
)]
fn run_delegated_backend(
    loaded_config: &LoadedConfig,
    tool: &'static str,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let backend_binary = find_backend_binary(tool);
    let output = ProcessCommand::new(&backend_binary)
        .args(["analyze", "--root"])
        .arg(repo_root)
        .args(["--format", "json"])
        .output()
        .map_err(|error| {
            CliError::backend_failure(format!(
                "{tool} failed to start for `{}`",
                repo_root.display()
            ))
            .with_source(error)
            .with_detail(consts::FIELD_TOOL, json!(tool))
            .with_detail(consts::FIELD_ROOT, json!(repo_root.display().to_string()))
            .with_detail("backend_path", json!(backend_binary.display().to_string()))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let cause = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("delegated backend `{tool}` exited non-zero")
        };
        return Err(CliError::backend_failure(format!(
            "{tool} failed to analyze `{}`",
            repo_root.display()
        ))
        .with_cause(cause)
        .with_detail(consts::FIELD_TOOL, json!(tool))
        .with_detail(consts::FIELD_ROOT, json!(repo_root.display().to_string()))
        .with_detail("backend_path", json!(backend_binary.display().to_string()))
        .with_detail("exit_status", json!(output.status.code())));
    }

    let raw = std::str::from_utf8(&output.stdout).map_err(|error| {
        CliError::backend_protocol(format!("{tool} returned non-utf8 machine output"))
            .with_source(error)
            .with_detail(consts::FIELD_TOOL, json!(tool))
            .with_suggested_action(
                "Run the backend binary directly and inspect its stdout encoding before rerunning sc-lint.",
            )
    })?;
    let normalized = normalize_backend_json(tool, raw)?;
    let finding_count = normalized
        .get(consts::FIELD_FINDINGS)
        .and_then(Value::as_array)
        .map_or(0, std::vec::Vec::len);

    Ok(CommandSuccess::with_dispatch(
        normalized,
        DispatchTelemetry::new(tool, finding_count),
    ))
}

fn find_backend_binary(tool: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|current_exe| find_backend_binary_from_exe(&current_exe, tool))
        // Falling back to PATH keeps local development, CI, and non-sibling
        // binary layouts working when the installed sibling lookup does not.
        .unwrap_or_else(|| PathBuf::from(tool))
}

fn find_backend_binary_from_exe(current_exe: &Path, tool: &str) -> Option<PathBuf> {
    let parent = current_exe.parent()?;
    find_backend_binary_in_dir(parent, tool).or_else(|| {
        parent
            .parent()
            .and_then(|grandparent| find_backend_binary_in_dir(grandparent, tool))
    })
}

fn find_backend_binary_in_dir(dir: &Path, tool: &str) -> Option<PathBuf> {
    let sibling = dir.join(tool);
    if sibling.is_file() {
        return Some(sibling);
    }
    if cfg!(windows) {
        let sibling_exe = dir.join(format!("{tool}.exe"));
        if sibling_exe.is_file() {
            return Some(sibling_exe);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::find_backend_binary_from_exe;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn prefers_installed_sibling_backend_binary() {
        let temp_dir = TempDir::new().expect("temp dir");
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("bin dir");
        let current_exe = bin_dir.join("sc-lint");
        let backend = bin_dir.join("sc-lint-portability");
        fs::write(&current_exe, "").expect("current exe");
        fs::write(&backend, "").expect("backend");

        assert_eq!(
            find_backend_binary_from_exe(&current_exe, "sc-lint-portability"),
            Some(backend)
        );
    }

    #[test]
    fn falls_back_to_target_debug_backend_binary_for_test_layouts() {
        let temp_dir = TempDir::new().expect("temp dir");
        let deps_dir = temp_dir.path().join("target").join("debug").join("deps");
        fs::create_dir_all(&deps_dir).expect("deps dir");
        let current_exe = deps_dir.join("sc-lint-tests");
        let backend = deps_dir
            .parent()
            .expect("target debug")
            .join("sc-lint-runtime");
        fs::write(&current_exe, "").expect("current exe");
        fs::write(&backend, "").expect("backend");

        assert_eq!(
            find_backend_binary_from_exe(&current_exe, "sc-lint-runtime"),
            Some(backend)
        );
    }
}
